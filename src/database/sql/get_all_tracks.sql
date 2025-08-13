SELECT * FROM tracks
ORDER BY
  tracks.album ASC,
  CAST(tracks.disc AS INTEGER) ASC,
  CAST(tracks.track AS INTEGER) ASC;
