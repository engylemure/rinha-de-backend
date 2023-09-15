CREATE OR REPLACE FUNCTION immutable_array_to_string(text [], text) RETURNS text as $$
SELECT array_to_string($1, $2);
$$ LANGUAGE sql IMMUTABLE;
CREATE TABLE IF NOT EXISTS PESSOAS (
    ID VARCHAR(36),
    APELIDO VARCHAR(32) CONSTRAINT ID_PK PRIMARY KEY,
    NOME VARCHAR(100),
    NASCIMENTO CHAR(10),
    STACK TEXT [],
    BUSCA_TRGM TEXT GENERATED ALWAYS AS (
        CASE
            WHEN STACK IS NULL THEN LOWER(NOME || APELIDO)
            ELSE LOWER(
                NOME || APELIDO || immutable_array_to_string(STACK, ' ')
            )
        END
    ) STORED
);
CREATE EXTENSION PG_TRGM;
CREATE INDEX CONCURRENTLY IF NOT EXISTS IDX_PESSOAS_BUSCA_TGRM ON PESSOAS USING GIST (BUSCA_TRGM GIST_TRGM_OPS);