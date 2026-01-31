CREATE TABLE IF NOT EXISTS notes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    year INTEGER NOT NULL,
    note_number INTEGER NOT NULL,
    body TEXT,
    UNIQUE(year, note_number)
);

INSERT INTO notes (year, note_number, body)
SELECT year, 1, value
FROM settings_year
WHERE key = 'note_1' AND value IS NOT NULL AND value <> ''
UNION ALL
SELECT year, 2, value
FROM settings_year
WHERE key = 'note_2' AND value IS NOT NULL AND value <> ''
UNION ALL
SELECT year, 3, value
FROM settings_year
WHERE key = 'note_3' AND value IS NOT NULL AND value <> '';
