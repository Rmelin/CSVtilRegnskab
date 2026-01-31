CREATE TABLE IF NOT EXISTS report_category_map (
    year INTEGER NOT NULL,
    group_id INTEGER NOT NULL,
    category_key TEXT NOT NULL,
    PRIMARY KEY (year, group_id)
);
