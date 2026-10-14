
ALTER TABLE photo_decisions ADD COLUMN book_mark TEXT NULL CHECK(book_mark IS NULL OR book_mark IN('spread','full'));
