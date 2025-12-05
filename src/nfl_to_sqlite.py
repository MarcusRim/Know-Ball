# nfl_to_sqlite.py
# End-to-end: nfl_data_py -> merge team/position -> SQLite (one row per player-season)

import argparse
import os
import sqlite3
import pandas as pd
import nfl_data_py as nfl

DB_PATH = "nfl.sqlite"
YEARS = list(range(2000, 2025))      # 2000–2024 inclusive

# -------------------------------
# SQLite setup
# -------------------------------
def init_db(path=DB_PATH):
    conn = sqlite3.connect(path)
    cur = conn.cursor()

    cur.execute("""
    CREATE TABLE IF NOT EXISTS players (
        player_id   TEXT PRIMARY KEY,
        name        TEXT,
        position    TEXT,
        college     TEXT,
        latest_team TEXT
    );
    """)

    cur.execute("""
    CREATE TABLE IF NOT EXISTS seasons (
        player_id           TEXT,
        season              INTEGER,
        team_abbr           TEXT,
        position            TEXT,
        -- Passing
        completions         INTEGER,
        attempts            INTEGER,
        passing_yards       INTEGER,
        passing_tds         INTEGER,
        interceptions       INTEGER,
        passer_rating       REAL,
        sacks               INTEGER,
        sack_yards          INTEGER,
        -- Rushing
        rushing_attempts    INTEGER,
        rushing_yards       INTEGER,
        rushing_tds         INTEGER,
        -- Receiving
        targets             INTEGER,
        receptions          INTEGER,
        receiving_yards     INTEGER,
        receiving_tds       INTEGER,
        -- Fumbles (combined)
        fumbles             INTEGER,
        fumbles_lost        INTEGER,
        -- Defense (not in seasonal -> NULLs)
        solo_tackles        INTEGER,
        assists             INTEGER,
        sacks_def           REAL,
        interceptions_def   INTEGER,
        -- Misc
        games               INTEGER,
        games_started       INTEGER,
        PRIMARY KEY (player_id, season),
        FOREIGN KEY (player_id) REFERENCES players(player_id)
    );
    """)
    conn.commit()
    return conn

# -------------------------------
# Helpers
# -------------------------------
def g(row, col):
    return row[col] if (col in row and pd.notna(row[col])) else None

def upsert_players(conn, roster_df):
    # Take most recent roster row per player to get latest team/position/college
    keep_cols = [c for c in ["player_id","player_name","position","college_name","team","season"] if c in roster_df.columns]
    r = roster_df[keep_cols].sort_values(["player_id","season"]).drop_duplicates("player_id", keep="last")

    rows = []
    for _, x in r.iterrows():
        rows.append((
            g(x,"player_id"),
            g(x,"player_name"),
            g(x,"position"),
            g(x,"college_name"),
            g(x,"team"),
        ))

    with conn:
        conn.executemany("""
            INSERT INTO players (player_id, name, position, college, latest_team)
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT(player_id) DO UPDATE SET
              name=excluded.name,
              position=excluded.position,
              college=excluded.college,
              latest_team=excluded.latest_team
        """, rows)

def insert_seasons(conn, seasonal_df):
    # limit to 2000–2024 just in case
    seasonal_df = seasonal_df[(seasonal_df["season"] >= 2000) & (seasonal_df["season"] <= 2024)].copy()

    rows = []
    for _, r in seasonal_df.iterrows():
        # combine fumbles across buckets that exist in this schema
        f_total = (g(r,"rushing_fumbles") or 0) + (g(r,"receiving_fumbles") or 0) + (g(r,"sack_fumbles") or 0)
        fl_total = (g(r,"rushing_fumbles_lost") or 0) + (g(r,"receiving_fumbles_lost") or 0) + (g(r,"sack_fumbles_lost") or 0)

        rows.append((
            g(r,"player_id"),
            int(r["season"]),
            g(r,"team"),                  # from roster merge
            g(r,"position"),              # from roster merge

            # Passing (note: no passer_rating in your seasonal -> insert None)
            g(r,"completions"),
            g(r,"attempts"),
            g(r,"passing_yards"),
            g(r,"passing_tds"),
            g(r,"interceptions"),
            None,                         # passer_rating not present in this dataset
            g(r,"sacks"),
            g(r,"sack_yards"),

            # Rushing
            g(r,"carries"),               # attempts = "carries" here
            g(r,"rushing_yards"),
            g(r,"rushing_tds"),

            # Receiving
            g(r,"targets"),
            g(r,"receptions"),
            g(r,"receiving_yards"),
            g(r,"receiving_tds"),

            # Fumbles combined
            f_total,
            fl_total,

            # Defense: not available here -> NULLs
            None,                         # solo_tackles
            None,                         # assists
            None,                         # sacks_def
            None,                         # interceptions_def

            # Misc
            g(r,"games"),
            g(r,"games_started")          # may be absent; stays NULL
        ))

    with conn:
        conn.executemany("""
            INSERT OR REPLACE INTO seasons
            (player_id, season, team_abbr, position,
             completions, attempts, passing_yards, passing_tds, interceptions, passer_rating, sacks, sack_yards,
             rushing_attempts, rushing_yards, rushing_tds,
             targets, receptions, receiving_yards, receiving_tds,
             fumbles, fumbles_lost, solo_tackles, assists, sacks_def, interceptions_def,
             games, games_started)
            VALUES (?,?,?,?, ?,?,?,?, ?,?, ?, ?, ?,?, ?, ?,?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        """, rows)

def process_team(team, conn, rosters_all, seasonal_all):
    # merge team/position from rosters, then filter this team
    keep = [c for c in ["player_id","season","team","position","player_name","college_name"] if c in rosters_all.columns]
    ro_small = rosters_all[keep].copy()

    seasonal = seasonal_all.merge(ro_small, on=["player_id","season"], how="left")
    seasonal_team = seasonal[seasonal["team"] == team].copy()
    rosters_team  = rosters_all[rosters_all["team"] == team].copy()

    if not seasonal_team.empty:
        upsert_players(conn, rosters_team)
        insert_seasons(conn, seasonal_team)

def main():
    parser = argparse.ArgumentParser(description="Build nfl.sqlite from nfl_data_py exports")
    parser.add_argument("--fresh", action="store_true", help="Remove existing DB before building")
    args = parser.parse_args()

    # Canonical current teams (32)
    teams = [
        "BUF","MIA","NE","NYJ","BAL","CIN","CLE","PIT",
        "HOU","IND","JAX","TEN","DEN","KC","LV","LAC",
        "DAL","NYG","PHI","WAS","CHI","DET","GB","MIN",
        "ATL","CAR","NO","TB","ARI","LAR","SF","SEA",
    ]

    years = list(range(2000, 2025))

    print(f"Loading rosters & seasonal for {years[0]}–{years[-1]} once ...")
    # Use the internal loaders your version exposes
    rosters_all  = nfl.__import_rosters("seasonal", years)
    seasonal_all = nfl.import_seasonal_data(years, "REG")

    # Normalize legacy team codes to current canonical abbreviations so we
    # don't need to perform a manual DB edit after import.
    TEAM_REMAP = {
        "OAK": "LV",
        "SD":  "LAC",
        "STL": "LAR",
        "LA":  "LAR",
    }

    if "team" in rosters_all.columns:
        rosters_all["team"] = rosters_all["team"].replace(TEAM_REMAP)
    if "team" in seasonal_all.columns:
        seasonal_all["team"] = seasonal_all["team"].replace(TEAM_REMAP)

    # Optionally remove existing DB for a fresh build
    if args.fresh and os.path.exists(DB_PATH):
        print(f"Removing existing DB at {DB_PATH} (fresh build)")
        os.remove(DB_PATH)

    conn = init_db(DB_PATH)  # OPEN DB ONCE

    for t in teams:
        print(f"Processing {t} ...")
        process_team(t, conn, rosters_all, seasonal_all)

    conn.close()
    print("✅ All teams done. Check the DB.")

if __name__ == "__main__":
    main()
