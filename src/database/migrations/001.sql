CREATE TABLE IF NOT EXISTS tracks(
  id INTEGER PRIMARY KEY,

  title TEXT,
  artist TEXT,
  genre TEXT,
  album TEXT,
  album_artist TEXT,
  disc TEXT,
  disc_total TEXT,
  track TEXT,
  track_total TEXT,
  duration INTEGER,

  modified DATETIME,

  path TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS track_file ON tracks(path);
