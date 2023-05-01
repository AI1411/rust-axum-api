CREATE TABLE IF NOT EXISTS labels
(
    id   SERIAL PRIMARY KEY,
    name TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS todo_labels
(
    id       SERIAL PRIMARY KEY,
    todo_id  INTEGER NOT NULL REFERENCES todos (id) DEFERRABLE INITIALLY DEFERRED,
    label_id INTEGER NOT NULL REFERENCES labels (id) DEFERRABLE INITIALLY DEFERRED
);