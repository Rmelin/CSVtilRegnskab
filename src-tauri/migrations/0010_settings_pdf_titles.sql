INSERT INTO settings_year (year, key, value)
SELECT year, 'pdf_title_line1', 'ÅLHOLM IDRÆTSFORENING'
FROM settings_year
GROUP BY year
ON CONFLICT(year, key) DO NOTHING;

INSERT INTO settings_year (year, key, value)
SELECT year, 'pdf_title_line2', 'VÆGTLØFTNINGSAFDELINGEN'
FROM settings_year
GROUP BY year
ON CONFLICT(year, key) DO NOTHING;
