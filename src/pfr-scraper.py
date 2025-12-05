# sample url: https://www.pro-football-reference.com/teams/pit/2013_roster.htm
teams = ['pit', 'bal', 'cin', 'cle', 'nwe', 'buf', 'mia', 'nyj', 'ind', 'jax', 'hou', 'ten', 'den', 'lac', 'kan', 'lvr', 
         'dal', 'nyg', 'phi', 'was', 'chi', 'det', 'gnb', 'min', 'car', 'atl', 'nor', 'tam', 'sfo', 'sea', 'ram', 'ari']
years = [i for i in range(2000, 2024)]

# plan is to iterate through each team and year, scrape player data if not already present
# save to a sqlite file

import time
import sqlite3
import re
import json
import requests
from bs4 import BeautifulSoup, Comment
from urllib.parse import urljoin

BASE_URL = "https://www.pro-football-reference.com"
HEADERS = {
    "User-Agent": (
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) "
        "AppleWebKit/537.36 (KHTML, like Gecko) "
        "Chrome/123.0.0.0 Safari/537.36"
    )
}

# setup SQLite
def init_db(db_path="pfr.sqlite"):
    conn = sqlite3.connect(db_path)
    cur = conn.cursor()

    # Players
    cur.execute("""
    CREATE TABLE IF NOT EXISTS players (
        player_id TEXT PRIMARY KEY,
        name TEXT,
        position TEXT,
        college TEXT,
        url TEXT
    );
    """)

    # Passing
    cur.execute("""
    CREATE TABLE IF NOT EXISTS seasons_passing (
        player_id TEXT,
        year INTEGER,
        age INTEGER,
        team TEXT,
        lg TEXT,
        pos TEXT,
        g INTEGER,
        gs INTEGER,
        qbrec TEXT,
        cmp INTEGER,
        att INTEGER,
        cmp_pct REAL,
        yds INTEGER,
        td INTEGER,
        td_pct REAL,
        int INTEGER,
        int_pct REAL,
        first_down INTEGER,
        succ_pct REAL,
        long INTEGER,
        y_per_att REAL,
        ay_per_att REAL,
        y_per_cmp REAL,
        y_per_g REAL,
        rate REAL,
        qbr REAL,
        sacks INTEGER,
        sack_yds INTEGER,
        sack_pct REAL,
        ny_per_att REAL,
        any_per_att REAL,
        four_q_comebacks INTEGER,
        gwd INTEGER,
        av INTEGER,
        PRIMARY KEY (player_id, year),
        FOREIGN KEY (player_id) REFERENCES players(player_id)
    );
    """)

    # Rushing & Receiving (store flexible per-row JSON)
    cur.execute("""
    CREATE TABLE IF NOT EXISTS seasons_rush_recv (
        player_id TEXT,
        year INTEGER,
        team TEXT,
        pos TEXT,
        row_json TEXT NOT NULL,
        PRIMARY KEY (player_id, year, team, pos),
        FOREIGN KEY (player_id) REFERENCES players(player_id)
    );
    """)

    # Defense & Fumbles (store flexible per-row JSON)
    cur.execute("""
    CREATE TABLE IF NOT EXISTS seasons_def_fum (
        player_id TEXT,
        year INTEGER,
        team TEXT,
        pos TEXT,
        row_json TEXT NOT NULL,
        PRIMARY KEY (player_id, year, team, pos),
        FOREIGN KEY (player_id) REFERENCES players(player_id)
    );
    """)

    conn.commit()
    return conn

# HTML helpers
def get_soup(url):
    r = requests.get(url, headers=HEADERS, timeout=30)
    r.raise_for_status()
    return BeautifulSoup(r.text, "lxml")

def find_comment_table(soup, table_id):
    """
    PFR often wraps tables in <!-- ... -->.
    We look inside div#all_<table_id> for an HTML comment containing the table,
    else fallback to direct <table id=...> if present.
    """
    wrapper = soup.find("div", id=f"all_{table_id}")
    if wrapper:
        for c in wrapper.children:
            if isinstance(c, Comment):
                inner = BeautifulSoup(c, "lxml")
                tbl = inner.find("table", id=table_id)
                if tbl:
                    return tbl
    return soup.find("table", id=table_id)

def text(el):
    return el.get_text(strip=True) if el else ""

def to_int(v):
    try:
        return int(v)
    except:
        return None

def to_float(v):
    try:
        return float(v)
    except:
        return None

# Roster page -> player links
def get_roster_player_links(roster_url):
    soup = get_soup(roster_url)
    roster_table = soup.find("table", id="roster") or find_comment_table(soup, "roster")
    if not roster_table:
        raise RuntimeError("Could not find roster table on roster page.")

    links = []
    for a in roster_table.select("tbody tr th[data-stat='player'] a"):
        name = text(a)
        rel = a.get("href")
        if rel:
            links.append((name, urljoin(BASE, rel)))
    return links

# Player meta (minimal)
PLAYER_ID_RE = re.compile(r"/players/([A-Z])/([A-Za-z0-9]+)\.htm$")

def normalize_player_id(url):
    m = PLAYER_ID_RE.search(url)
    return f"{m.group(1)}/{m.group(2)}" if m else url

def scrape_player_meta(soup):
    """
    Return dict: {position, college}. Name is taken from page header when possible.
    """
    meta = {"position": None, "college": None, "name": None}

    # Name from h1
    h1 = soup.find("h1", {"itemprop": "name"})
    if h1:
        meta["name"] = text(h1)

    meta_div = soup.find("div", id="meta")
    if meta_div:
        # Position
        pos_anchor = meta_div.find(string=re.compile(r"Position:"))
        if pos_anchor and pos_anchor.parent:
            strong = pos_anchor.parent.find_next("strong")
            if strong:
                meta["position"] = text(strong)

        # College
        college_anchor = meta_div.find(string=re.compile(r"College:"))
        if college_anchor and college_anchor.parent:
            link = college_anchor.parent.find_next("a")
            meta["college"] = text(link)

    return meta

# Table row extraction
def iter_table_rows(tbl):
    """Yield dict keyed by 'data-stat' from tbody rows (skip header/totals)."""
    if not tbl:
        return
    for tr in tbl.select("tbody tr"):
        # Skip extra header rows or summary rows
        if tr.get("class") and "thead" in tr.get("class"):
            continue
        rd = {}
        for td in tr.find_all(["th", "td"]):
            stat = td.get("data-stat")
            if not stat:
                continue
            rd[stat] = text(td)
        # Only keep if a numeric year
        y = rd.get("year_id")
        if y and y.isdigit():
            y_int = int(y)
            if YEAR_MIN <= y_int <= YEAR_MAX:
                yield rd

# Passing mapping (exact fields)
# Map desired output columns -> PFR data-stat keys
PASSING_MAP = {
    "year":        "year_id",
    "age":         "age",
    "team":        "team",
    "lg":          "lg",
    "pos":         "pos",
    "g":           "g",
    "gs":          "gs",
    "qbrec":       "qb_rec",
    "cmp":         "pass_cmp",
    "att":         "pass_att",
    "cmp_pct":     "pass_cmp_perc",
    "yds":         "pass_yds",
    "td":          "pass_td",
    "td_pct":      "pass_td_perc",
    "int":         "pass_int",
    "int_pct":     "pass_int_perc",
    "first_down":  "pass_first_down",        # 1D
    "succ_pct":    "pass_success_perc",      # Succ% (may be missing historically)
    "long":        "pass_long",              # Lng
    "y_per_att":   "pass_yds_per_att",       # Y/A
    "ay_per_att":  "pass_adj_yds_per_att",   # AY/A
    "y_per_cmp":   "pass_yds_per_cmp",       # Y/C
    "y_per_g":     "pass_yds_per_g",         # Y/G
    "rate":        "pass_rating",            # Rate
    "qbr":         "qbr",                    # ESPN QBR (may be blank for early years)
    "sacks":       "pass_sacked",            # Sk
    "sack_yds":    "pass_sacked_yds",        # Yds (lost)
    "sack_pct":    "pass_sacked_perc",       # Sk%
    "ny_per_att":  "pass_net_yds_per_att",   # NY/A
    "any_per_att": "pass_adj_net_yds_per_att", # ANY/A
    "four_q_comebacks": "comebacks",         # 4QC
    "gwd":              "gwd",               # GWD
    "av":               "av",                # AV
}

def coerce_passing_row(rd):
    """Return tuple in the order of seasons_passing columns, coercing types."""
    def g(key, default=None): return rd.get(key, default)

    return (
        # year handled outside
        None,  # placeholder for player_id in insert
        to_int(g("year_id")),
        to_int(g("age")),
        g("team"),
        g("lg"),
        g("pos"),
        to_int(g("g")),
        to_int(g("gs")),
        g("qb_rec"),
        to_int(g("pass_cmp")),
        to_int(g("pass_att")),
        to_float(g("pass_cmp_perc")),
        to_int(g("pass_yds")),
        to_int(g("pass_td")),
        to_float(g("pass_td_perc")),
        to_int(g("pass_int")),
        to_float(g("pass_int_perc")),
        to_int(g("pass_first_down")),
        to_float(g("pass_success_perc")),
        to_int(g("pass_long")),
        to_float(g("pass_yds_per_att")),
        to_float(g("pass_adj_yds_per_att")),
        to_float(g("pass_yds_per_cmp")),
        to_float(g("pass_yds_per_g")),
        to_float(g("pass_rating")),
        to_float(g("qbr")),
        to_int(g("pass_sacked")),
        to_int(g("pass_sacked_yds")),
        to_float(g("pass_sacked_perc")),
        to_float(g("pass_net_yds_per_att")),
        to_float(g("pass_adj_net_yds_per_att")),
        to_int(g("comebacks")),
        to_int(g("gwd")),
        to_int(g("av")),
    )

# Scrape a player page
def scrape_player(player_url):
    soup = get_soup(player_url)

    meta = scrape_player_meta(soup)
    name = meta.get("name")
    position = meta.get("position")
    college = meta.get("college")

    # Tables
    passing_tbl   = find_comment_table(soup, "passing")
    rushrec_tbl   = find_comment_table(soup, "rushing_and_receiving")
    defense_tbl   = find_comment_table(soup, "defense")
    fumbles_tbl   = find_comment_table(soup, "fumbles")

    passing_rows = list(iter_table_rows(passing_tbl))
    rushrec_rows = list(iter_table_rows(rushrec_tbl))
    defense_rows = list(iter_table_rows(defense_tbl))
    fumbles_rows = list(iter_table_rows(fumbles_tbl))

    # Join defense & fumbles by (year, team, pos) if you prefer; here we store both rows independently
    return {
        "name": name,
        "position": position,
        "college": college,
        "passing_rows": passing_rows,
        "rushrec_rows": rushrec_rows,
        "defense_rows": defense_rows,
        "fumbles_rows": fumbles_rows,
    }

# DB writers
def upsert_player(cur, player_id, name, position, college, url):
    cur.execute("""
        INSERT INTO players (player_id, name, position, college, url)
        VALUES (?, ?, ?, ?, ?)
        ON CONFLICT(player_id) DO UPDATE SET
            name=excluded.name,
            position=excluded.position,
            college=excluded.college,
            url=excluded.url
    """, (player_id, name, position, college, url))

def insert_passing(cur, player_id, rows):
    for rd in rows:
        yr = to_int(rd.get("year_id"))
        if yr is None or yr < YEAR_MIN or yr > YEAR_MAX:
            continue
        # Build tuple and replace placeholder with player_id
        vals = list(coerce_passing_row(rd))
        vals[0] = player_id  # set player_id into first slot
        cur.execute("""
            INSERT OR REPLACE INTO seasons_passing
            (player_id, year, age, team, lg, pos, g, gs, qbrec, cmp, att, cmp_pct, yds, td, td_pct,
             int, int_pct, first_down, succ_pct, long, y_per_att, ay_per_att, y_per_cmp, y_per_g,
             rate, qbr, sacks, sack_yds, sack_pct, ny_per_att, any_per_att, four_q_comebacks, gwd, av)
            VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)
        """, tuple(vals))

def insert_rush_recv(cur, player_id, rows):
    for rd in rows:
        yr = to_int(rd.get("year_id"))
        if yr is None or yr < YEAR_MIN or yr > YEAR_MAX:
            continue
        team = rd.get("team")
        pos  = rd.get("pos")
        cur.execute("""
            INSERT OR REPLACE INTO seasons_rush_recv
            (player_id, year, team, pos, row_json)
            VALUES (?, ?, ?, ?, ?)
        """, (player_id, yr, team, pos, json.dumps(rd, separators=(",", ":"))))

def insert_def_fum(cur, player_id, defense_rows, fumbles_rows):
    # Store defense rows
    for rd in defense_rows:
        yr = to_int(rd.get("year_id"))
        if yr is None or yr < YEAR_MIN or yr > YEAR_MAX:
            continue
        team = rd.get("team")
        pos  = rd.get("pos")
        cur.execute("""
            INSERT OR REPLACE INTO seasons_def_fum
            (player_id, year, team, pos, row_json)
            VALUES (?, ?, ?, ?, ?)
        """, (player_id, yr, team, pos, json.dumps(rd, separators=(",", ":"))))

    # Store fumbles rows (kept separate; if you prefer merging with defense by (year,team,pos), do it here)
    for rd in fumbles_rows:
        yr = to_int(rd.get("year_id"))
        if yr is None or yr < YEAR_MIN or yr > YEAR_MAX:
            continue
        team = rd.get("team")
        pos  = rd.get("pos")
        # To distinguish fumbles vs defense if colliding keys, you could add a marker into JSON
        rd_marked = dict(rd)
        rd_marked["_source"] = "fumbles"
        cur.execute("""
            INSERT OR REPLACE INTO seasons_def_fum
            (player_id, year, team, pos, row_json)
            VALUES (?, ?, ?, ?, ?)
        """, (player_id, yr, team, pos, json.dumps(rd_marked, separators=(",", ":"))))

# Driver
def scrape_roster_to_sqlite(roster_url, db_path="pfr.sqlite", max_players=None, delay=1.0):
    conn = init_db(db_path)
    cur = conn.cursor()

    players = get_roster_player_links(roster_url)
    if max_players:
        players = players[:max_players]
    print(f"Found {len(players)} players.")

    for i, (name_on_roster, url) in enumerate(players, 1):
        pid = normalize_player_id(url)
        print(f"[{i}/{len(players)}] {name_on_roster} -> {pid}")
        try:
            data = scrape_player(url)

            # Prefer the page header name; fallback to roster name
            name = data["name"] or name_on_roster
            position = data["position"]
            college = data["college"]

            upsert_player(cur, pid, name, position, college, url)

            insert_passing(cur, pid, data["passing_rows"])
            insert_rush_recv(cur, pid, data["rushrec_rows"])
            insert_def_fum(cur, pid, data["defense_rows"], data["fumbles_rows"])

            conn.commit()
        except Exception as e:
            print(f"  !! Error for {name_on_roster}: {e}")
        time.sleep(delay)  # be polite

    conn.close()

if __name__ == "__main__":
    # Example: 2013 Steelers roster
    scrape_roster_to_sqlite(
        "https://web.archive.org/web/20240202161509/https://www.pro-football-reference.com/teams/pit/2013_roster.htm",
        db_path="pfr.sqlite",
        max_players=None,
        delay=1.0
    )