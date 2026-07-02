-- Shared sample query for client-library examples (public sample schema).
SELECT SingerId, AlbumId, AlbumTitle
FROM Albums
WHERE STARTS_WITH(AlbumTitle, 'A')
LIMIT 10
