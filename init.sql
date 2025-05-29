CREATE TABLE
    IF NOT EXISTS pools (
        protocol TEXT NOT NULL,
        factory TEXT NOT NULL,
        pool TEXT NOT NULL,
        token0 TEXT NOT NULL,
        token1 TEXT NOT NULL,
        fee INT NOT NULL,
        token0_liquidity INT NOT NULL,
        token1_liquidity INT NOT NULL,
        timestamp INT NOT NULL,
        PRIMARY KEY (pool)
    );