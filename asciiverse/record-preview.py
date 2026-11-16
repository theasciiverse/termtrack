#!/usr/bin/env python3
"""Records asciiverse/termtrack.cast, the catalog preview: 100x30, a run
through a level played by a route bot.

The game runs in a pty that answers like a terminal with the kitty keyboard
protocol, so the bot presses and releases keys exactly as a player does. It
steers by the player's position, read from the game's own session log
(TERMTRACK_LOG with RUST_LOG=trace logs every frame).

TermTrack redraws the whole screen in RGB a few hundred times a second, so
the cast is not the raw output, which goes to a file as it comes: a small
screen model reads that file back holding only the last two redraws, and
the cast holds the screen 10 times a second, only the cells that changed,
with RGB colours rounded to steps of 7 (asciicast v3, as asciinema 3 writes
it, hub/RECORDING.md). That keeps it under 2 MB and the recorder's memory
at a few tens of MB.

    cargo build --release
    python3 asciiverse/record-preview.py [--route level_1] [--out asciiverse/termtrack.cast]
    python3 asciiverse/record-preview.py --cast-only --raw <raw file> [--out ...]

--serve FIFO runs the game and takes bot commands (Python expressions on
`bot`) from FIFO, one per line, for working out a route; `bot.shot(png)`
draws the screen (needs Pillow).
"""
import argparse, fcntl, json, math, os, pty, re, select, shutil, struct, sys
import tempfile, termios, threading, time

COLS, ROWS = 100, 30
HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)

# ---------------------------------------------------------------- screen

class Screen:
    """Just enough of a terminal for what TermTrack prints."""
    def __init__(self, cols, rows):
        self.cols, self.rows = cols, rows
        self.blank = (" ", None, None)
        self.cells = [[self.blank] * cols for _ in range(rows)]
        self.x = self.y = 0
        self.fg = self.bg = None
        self.buf = b""

    def feed(self, data):
        self.buf += data
        text = self.buf.decode("utf-8", "ignore")
        # keep an unfinished escape or UTF-8 sequence for the next call
        cut = len(text)
        esc = text.rfind("\x1b")
        if esc != -1 and not re.match(r"\x1b(\[[0-9;:?<>=]*[@-~]|[^\[])", text[esc:]):
            cut = esc
        consumed = text[:cut]
        self.buf = text[cut:].encode()
        i = 0
        for m in re.finditer(r"\x1b\[([0-9;:?<>=]*)([@-~])|\x1b(.)|([\r\n\x08])|([^\x1b\r\n\x08])", consumed):
            if m.group(2):
                self.csi(m.group(1), m.group(2))
            elif m.group(4):
                c = m.group(4)
                if c == "\r": self.x = 0
                elif c == "\n": self.lf()
                elif c == "\x08": self.x = max(0, self.x - 1)
            elif m.group(5):
                c = m.group(5)
                if ord(c) < 32: continue
                if self.x >= self.cols:
                    self.x = 0; self.lf()
                self.cells[self.y][self.x] = (c, self.fg, self.bg)
                self.x += 1

    def lf(self):
        if self.y == self.rows - 1:
            self.cells.pop(0)
            self.cells.append([(" ", None, self.bg)] * self.cols)
        else:
            self.y += 1

    def csi(self, params, final):
        if params.startswith(("?", "<", ">", "=")):
            return
        p = [int(v) if v.isdigit() else 0 for v in params.replace(":", ";").split(";")] if params else []
        if final in "Hf":
            r = (p[0] if len(p) > 0 and p[0] else 1) - 1
            c = (p[1] if len(p) > 1 and p[1] else 1) - 1
            self.y = min(max(r, 0), self.rows - 1); self.x = min(max(c, 0), self.cols - 1)
        elif final == "J":
            mode = p[0] if p else 0
            if mode == 2 or mode == 3:
                self.cells = [[(" ", None, self.bg)] * self.cols for _ in range(self.rows)]
            elif mode == 0:
                self.cells[self.y][self.x:] = [(" ", None, self.bg)] * (self.cols - self.x)
                for r in range(self.y + 1, self.rows):
                    self.cells[r] = [(" ", None, self.bg)] * self.cols
        elif final == "K":
            self.cells[self.y][self.x:] = [(" ", None, self.bg)] * (self.cols - self.x)
        elif final == "m":
            if not p: p = [0]
            i = 0
            while i < len(p):
                v = p[i]
                if v == 0: self.fg = self.bg = None
                elif v in (38, 48) and i + 1 < len(p) and p[i + 1] == 2 and i + 4 < len(p):
                    col = tuple(p[i + 2:i + 5])
                    if v == 38: self.fg = col
                    else: self.bg = col
                    i += 4
                elif v in (38, 48) and i + 1 < len(p) and p[i + 1] == 5 and i + 2 < len(p):
                    if v == 38: self.fg = p[i + 2]
                    else: self.bg = p[i + 2]
                    i += 2
                elif v in (38, 48):
                    i += 1  # a broken colour, skipped as a terminal would
                elif v == 39: self.fg = None
                elif v == 49: self.bg = None
                elif 30 <= v <= 37: self.fg = v - 30
                elif 40 <= v <= 47: self.bg = v - 40
                elif 90 <= v <= 97: self.fg = v - 90 + 8
                elif 100 <= v <= 107: self.bg = v - 100 + 8
                i += 1

    def snapshot(self):
        return [row[:] for row in self.cells]

def colour(col, base, ext, dflt):
    if col is None: return str(dflt)
    if isinstance(col, tuple): return f"{ext};2;{col[0]};{col[1]};{col[2]}"
    if col < 8: return str(base + col)
    return f"{ext};5;{col}"

def sgr(fg, bg, attrs):
    """Escape code from attributes `attrs` (fg, bg, or None) to (fg, bg)."""
    if attrs is None:
        return "\x1b[0;" + colour(fg, 30, 38, 39) + ";" + colour(bg, 40, 48, 49) + "m"
    parts = []
    if fg != attrs[0]: parts.append(colour(fg, 30, 38, 39))
    if bg != attrs[1]: parts.append(colour(bg, 40, 48, 49))
    return "\x1b[" + ";".join(parts) + "m" if parts else ""

def quantise(snap, q):
    """Rounds RGB colours to steps of q, and drops the colour of blank text."""
    def r(c):
        if isinstance(c, tuple) and q > 1:
            return tuple(min(255, int(v / q + 0.5) * q) for v in c)
        return c
    return [[(ch, None if ch == " " else r(fg), r(bg)) for (ch, fg, bg) in row] for row in snap]

def diff(old, new):
    """Escape codes that turn screen `old` into `new`, changed cells only."""
    out, attrs = [], None
    for y in range(len(new)):
        x = 0
        row_new, row_old = new[y], (old[y] if old else None)
        while x < len(row_new):
            if row_old is not None and row_new[x] == row_old[x]:
                x += 1; continue
            # a run of changes; small unchanged gaps are resent, cheaper
            # than moving the cursor
            last, end = x, x + 1
            while end < len(row_new) and end - last <= 6:
                if row_old is None or row_new[end] != row_old[end]:
                    last = end
                end += 1
            end = last + 1
            out.append(f"\x1b[{y + 1};{x + 1}H")
            for c in range(x, end):
                ch, fg, bg = row_new[c]
                if attrs is None or bg != attrs[1] or (ch != " " and fg != attrs[0]):
                    if ch == " " and attrs is not None: fg = attrs[0]
                    out.append(sgr(fg, bg, attrs)); attrs = (fg, bg)
                out.append(ch)
            x = end
    return "".join(out)

def trim(buf, cap=4 << 20):
    """Every screen of TermTrack is a full redraw that starts with a bare
    ESC [ H, so the screen is the last finished redraw with whatever of the
    next one is out: everything before the last two is dropped."""
    last = buf.rfind(b"\x1b[H")
    if last > 0:
        prev = buf.rfind(b"\x1b[H", 0, last)
        if prev > 0:
            del buf[:prev]
    if len(buf) > cap:
        del buf[:len(buf) - cap]

def cast(raw, path, title, fps=10, q=7, end_pad=1.0):
    """Writes the cast from the raw output file: the screen sampled `fps`
    times a second, changed cells only. Streams the file record by record."""
    with open(raw, "rb") as f, open(path, "w") as out:
        head = {"version": 3, "term": {"cols": COLS, "rows": ROWS, "type": "xterm-256color"},
                "timestamp": int(time.time()), "idle_time_limit": 2.0, "title": title,
                "env": {"COLORTERM": "truecolor", "LANG": "C.UTF-8", "TERM": "xterm-256color"}}
        out.write(json.dumps(head, separators=(",", ":")) + "\n")
        buf = bytearray()
        state = {"prev": None, "prev_t": 0.0, "first": True}
        def sample(t):
            # the last finished redraw only, no tearing from the next one;
            # all of it once the output has been quiet a moment
            last = buf.rfind(b"\x1b[H")
            quiet = t - state.get("last_ct", 0.0) > 0.1
            frame = buf[:last] if last > 0 and not quiet else buf
            screen = Screen(COLS, ROWS)
            screen.feed(bytes(frame))
            snap = quantise(screen.snapshot(), q)
            o = diff(state["prev"], snap)
            if state["first"]:
                o = "\x1b[?25l\x1b[0m\x1b[2J" + o; state["first"] = False
            if o: o += "\x1b[0m"
            if o:
                out.write(json.dumps([round(t - state["prev_t"], 3), "o", o], ensure_ascii=False) + "\n")
                state["prev_t"], state["prev"] = t, snap
        t, step, last_ct = 0.0, 1.0 / fps, 0.0
        while True:
            head_b = f.read(12)
            if len(head_b) < 12: break
            ct, n = struct.unpack("<dI", head_b)
            data = f.read(n)
            while t < ct:
                sample(t); t += step
            buf += data
            trim(buf)
            last_ct = state["last_ct"] = ct
        end = last_ct + end_pad
        while t <= end:
            sample(t); t += step
        out.write(json.dumps([round(end - state["prev_t"], 3), "o", "\x1b[?25h"]) + "\n")
        out.write(json.dumps([0.0, "x", "0"]) + "\n")

# ---------------------------------------------------------------- game in a pty

KITTY = {"w": 119, "a": 97, "s": 115, "d": 100, "space": 32, "enter": 13,
         "e": 101, "m": 109, "r": 114, "l": 108, "esc": 27}
ARROWS = {"up": "A", "down": "B", "right": "C", "left": "D"}

class Game:
    def __init__(self, exe, cwd, log, raw):
        # the raw output goes to a file as it comes, (time, length, bytes)
        # records; memory keeps only the last two redraws, for shots
        self.raw = open(raw, "wb")
        self.tail = bytearray()
        self.lock = threading.Lock()
        self.log, self.log_pos, self.frame, self.frame_at = log, 0, None, time.time()
        self.home = tempfile.mkdtemp()
        env = {"PATH": "/usr/bin:/bin", "HOME": self.home, "USER": "player",
               "LOGNAME": "player", "TERM": "xterm-256color", "COLORTERM": "truecolor",
               "LANG": "C.UTF-8", "XDG_STATE_HOME": os.path.join(self.home, "state"),
               "TERMTRACK_LOG": log, "RUST_LOG": "trace"}
        pid, fd = pty.fork()
        if pid == 0:
            os.chdir(cwd)
            os.execve(exe, [exe], env)
        self.pid, self.fd = pid, fd
        fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", ROWS, COLS, 0, 0))
        self.t0 = time.time()
        self.alive = True
        threading.Thread(target=self.reader, daemon=True).start()

    def reader(self):
        while True:
            try:
                data = os.read(self.fd, 1 << 16)
            except OSError:
                break
            if not data: break
            now = time.time() - self.t0
            if b"\x1b[?u" in data:  # crossterm asks for the kitty protocol
                os.write(self.fd, b"\x1b[?0u\x1b[?62;22c")
            with self.lock:
                self.raw.write(struct.pack("<dI", now, len(data)) + data)
                self.tail += data
                trim(self.tail)
        self.alive = False
        with self.lock:
            self.raw.close()

    def send(self, s):
        os.write(self.fd, s.encode())

    def press(self, key):
        if key in ARROWS: self.send(f"\x1b[1;1:1{ARROWS[key]}")
        else: self.send(f"\x1b[{KITTY[key]}u")

    def release(self, key):
        if key in ARROWS: self.send(f"\x1b[1;1:3{ARROWS[key]}")
        else: self.send(f"\x1b[{KITTY[key]};1:3u")

    def tap(self, key, hold=0.06, after=0.25):
        self.press(key); time.sleep(hold); self.release(key); time.sleep(after)

    def state(self):
        """The newest frame line of the log: x, y, z, turn, pitch, grounded."""
        try:
            with open(self.log) as f:
                f.seek(self.log_pos)
                text = f.read()
                self.log_pos = f.tell()
        except FileNotFoundError:
            return self.frame
        for line in text.splitlines():
            m = re.search(r"frame: pos (\S+) (\S+) (\S+) turn (\S+) (\S+) grounded (\w+) time (\S+)", line)
            if m:
                v = m.groups()
                self.frame_at = time.time()
                self.frame = dict(x=float(v[0]), y=float(v[1]), z=float(v[2]), turn=float(v[3]),
                                  pitch=float(v[4]), grounded=v[5] == "true", time=float(v[6]))
        return self.frame

    def wait(self, t):
        time.sleep(t)

    def shot(self, png):
        from PIL import Image, ImageDraw
        cells = self.screen_at()
        img = Image.new("RGB", (COLS * 6, ROWS * 12)); d = ImageDraw.Draw(img)
        pal = [(0, 0, 0), (205, 0, 0), (0, 205, 0), (205, 205, 0), (0, 0, 238), (205, 0, 205), (0, 205, 205), (229, 229, 229)]
        def rgb(c, dflt):
            if c is None: return dflt
            if isinstance(c, tuple): return c
            return pal[c % 8]
        for y, row in enumerate(cells):
            for x, (ch, fg, bg) in enumerate(row):
                d.rectangle([x * 6, y * 12, x * 6 + 5, y * 12 + 11], fill=rgb(bg, (0, 0, 0)))
                if ch != " ": d.text((x * 6, y * 12), ch, fill=rgb(fg, (229, 229, 229)))
        img.save(png)

    def screen_at(self, t=None):
        """The screen now."""
        with self.lock:
            data = bytes(self.tail)
        screen = Screen(COLS, ROWS)
        screen.feed(data)
        return screen.snapshot()

    def close(self):
        try: os.kill(self.pid, 9)
        except OSError: pass
        try: os.waitpid(self.pid, 0)
        except OSError: pass
        shutil.rmtree(self.home, ignore_errors=True)

# ---------------------------------------------------------------- the bot

GW = 10.0

def cell(r, c):
    return (c * GW + GW / 2, r * GW + GW / 2)

class Bot:
    def __init__(self, game):
        self.g = game

    def s(self):
        for _ in range(200):
            st = self.g.state()
            if st: return st
            time.sleep(0.01)
        raise RuntimeError("no frames in the log")

    def heading(self, x, z):
        st = self.s()
        return math.atan2(x - st["x"], z - st["z"])

    @staticmethod
    def err(a, b):
        return (a - b + math.pi) % (2 * math.pi) - math.pi

    def turn_to(self, h, tol=0.03):
        """Turns on the spot to heading h (radians, 0 = +z, pi/2 = +x)."""
        for _ in range(3):
            e = self.err(h, self.s()["turn"])
            if abs(e) < tol: return
            key = "right" if e > 0 else "left"
            self.g.press(key)
            t_end = time.time() + abs(e) / 2.5 + 0.5
            while time.time() < t_end:
                e = self.err(h, self.s()["turn"])
                if (key == "right" and e < 0.012) or (key == "left" and e > -0.012): break
                time.sleep(0.002)
            self.g.release(key)
            time.sleep(0.05)

    def look(self, pitch, tol=0.03):
        for _ in range(3):
            e = pitch - self.s()["pitch"]
            if abs(e) < tol: return
            key = "up" if e > 0 else "down"
            self.g.press(key)
            t_end = time.time() + abs(e) / 2.5 + 0.5
            while time.time() < t_end:
                e = pitch - self.s()["pitch"]
                if (key == "up" and e < 0.012) or (key == "down" and e > -0.012): break
                time.sleep(0.002)
            self.g.release(key)
            time.sleep(0.05)

    def steer(self, x, z, held):
        """While walking: holds left or right to keep on course for (x, z)."""
        e = self.err(self.heading(x, z), self.s()["turn"])
        want = "right" if e > 0.04 else "left" if e < -0.04 else None
        for k in ("left", "right"):
            if k in held and k != want:
                self.g.release(k); held.discard(k)
        if want and want not in held:
            self.g.press(want); held.add(want)

    def go(self, r, c, jump_at=None, stop=1.0, turn_first=True, timeout=8.0, lead=0.0):
        """Walks to the middle of cell (r, c). jump_at: jump when this far
        from the target. lead: stop walking this far before it."""
        x, z = cell(r, c)
        if turn_first and (jump_at is not None
                           or abs(self.err(self.heading(x, z), self.s()["turn"])) > 0.5):
            self.turn_to(self.heading(x, z))
        held = set()
        self.g.press("w")
        jumped = False
        t_end = time.time() + timeout
        try:
            while time.time() < t_end:
                st = self.s()
                if time.time() - self.g.frame_at > 0.3:
                    break                   # the level is over (the end, or a fall)
                dist = math.hypot(x - st["x"], z - st["z"])
                if jump_at is not None and not jumped and dist <= jump_at:
                    self.g.press("space"); time.sleep(0.05); self.g.release("space")
                    jumped = True
                if dist <= stop + lead and (jumped is False or jump_at is None or st["grounded"] or dist <= stop):
                    break
                if dist > 2.0:
                    self.steer(x, z, held)
                time.sleep(0.003)
        finally:
            self.g.release("w")
            for k in held: self.g.release(k)
        time.sleep(0.08)
        return self.s()

    def settle(self, t=1.5):
        """Waits until the player stands still on something."""
        end = time.time() + t
        last = None
        while time.time() < end:
            y = self.s()["y"]
            if last is not None and abs(y - last) < 0.01: return
            last = y
            time.sleep(0.05)

    def jump(self, hold=0.05):
        self.g.press("space"); time.sleep(hold); self.g.release("space")

# ---------------------------------------------------------------- routes

def route_level_1(bot, upto=None):
    """level_1: across the bottom floor, down the line of pillars and
    stones on its east side, up to the middle floor and the stepping stones
    of the top one, and back down to the end."""
    b = bot
    steps = [
        # (row, col, jump when this far from the target)
        (0, 4, None), (4, 4, None), (4, 8, None), (0, 8, None), (0, 15, None),
        (0, 16, 7),            # onto the half wall
        (2, 16, 16), (4, 16, 16), (5, 15, 12), (6, 16, 12),
        (7, 16, 8),            # the half wall below the middle floor
        (7, 15, 9),            # up to the middle floor
        (7, 14, 8),            # its half wall
        (7, 12, 17),           # up to the top floor's stones
        (7, 9, 27), (7, 6, 27), (7, 3, 27),
        (7, 2, None),          # down onto the middle floor's half wall
        (4, 2, 27),            # over the holes to the middle floor
        (4, 0, None),
        (2, 0, 17), (0, 0, 17),   # and the end
    ]
    for i, (r, c, j) in enumerate(steps):
        if upto is not None and i >= upto: break
        b.go(r, c, jump_at=j, stop=1.0)
        if j is not None:
            b.settle()
        if (r, c) == (7, 12):
            b.look(-0.45)          # look down at the stones
        if (r, c) == (4, 2):
            b.look(0.0)

ROUTES = {"level_1": route_level_1}

# ---------------------------------------------------------------- main

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--exe", default=os.path.join(ROOT, "target", "release", "termtrack"))
    ap.add_argument("--route", default="level_1")
    ap.add_argument("--out", default=os.path.join(HERE, "termtrack.cast"))
    ap.add_argument("--serve", help="a FIFO to read bot commands from")
    ap.add_argument("--raw", help="keep the raw output here; with --cast-only, "
                    "make the cast from it")
    ap.add_argument("--cast-only", action="store_true")
    args = ap.parse_args()
    if args.cast_only:
        cast(args.raw, args.out, "TermTrack")
        print(args.out, os.path.getsize(args.out), "bytes")
        return
    tmp = tempfile.mkdtemp()
    log = os.path.join(tmp, "termtrack.log")
    raw = args.raw or os.path.join(tmp, "raw")
    game = Game(os.path.abspath(args.exe), ROOT, log, raw)
    bot = Bot(game)
    try:
        if args.serve:
            if not os.path.exists(args.serve): os.mkfifo(args.serve)
            while game.alive:
                with open(args.serve) as f:
                    for line in f:
                        line = line.strip()
                        if not line: continue
                        try:
                            res = eval(line, {**globals(), "bot": bot, "game": game})
                        except Exception as e:
                            res = f"error: {e!r}"
                        with open(args.serve + ".out", "a") as o:
                            o.write(f"{line} -> {res}\n")
            return
        time.sleep(1.5)                       # the menu
        level = sorted(os.listdir(os.path.join(ROOT, "levels"))).index(args.route + ".txt")
        for _ in range(level):
            game.tap("down", after=0.35)
        game.tap("enter", after=1.0)
        ROUTES[args.route](bot)
        time.sleep(2.5)                       # the finish screen
        game.tap("down", after=0.5); game.tap("enter", after=0.9)   # NO: to the menu
        game.tap("e", after=0.6); game.tap("enter", after=0.6)      # exit, YES
        for _ in range(50):
            if not game.alive: break
            time.sleep(0.1)
    finally:
        game.close()
    cast(raw, args.out, "TermTrack")
    print(args.out, os.path.getsize(args.out), "bytes")
    shutil.rmtree(tmp, ignore_errors=True)

if __name__ == "__main__":
    main()
