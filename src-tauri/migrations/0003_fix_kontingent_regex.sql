UPDATE matcher_rules
SET regex_pattern = '^\s*(\d{6,10})\s+(.+?)\s+-SE\s+MEDD\.\s*$'
WHERE name = 'Kontingent - SE MEDD.';
