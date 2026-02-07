-- Report: Check data existence for all months in 2025
-- Table: TEMPO.STATEMENT_LINE SUBPARTITION (EL_USA_SP1)
-- Conditions: processing_office_id = 38 AND IN (1,2,5,6,7,16)

SELECT 
    '2025-01' AS month_period,
    CASE WHEN EXISTS (
        SELECT /*+ FIRST_ROWS(1) NO_PARALLEL */ 1 
        FROM TEMPO.STATEMENT_LINE SUBPARTITION (EL_USA_SP1) sl
        WHERE sl.processing_office_id = 38
          AND sl.processing_office_id IN (1,2,5,6,7,16)
          AND sl.accounting_period_dt = DATE '2025-01-31'
          AND ROWNUM = 1
    ) THEN 'Y' ELSE 'N' END AS data_exists
FROM DUAL
UNION ALL
SELECT 
    '2025-02' AS month_period,
    CASE WHEN EXISTS (
        SELECT /*+ FIRST_ROWS(1) NO_PARALLEL */ 1 
        FROM TEMPO.STATEMENT_LINE SUBPARTITION (EL_USA_SP1) sl
        WHERE sl.processing_office_id = 38
          AND sl.processing_office_id IN (1,2,5,6,7,16)
          AND sl.accounting_period_dt = DATE '2025-02-28'
          AND ROWNUM = 1
    ) THEN 'Y' ELSE 'N' END AS data_exists
FROM DUAL
UNION ALL
SELECT 
    '2025-03' AS month_period,
    CASE WHEN EXISTS (
        SELECT /*+ FIRST_ROWS(1) NO_PARALLEL */ 1 
        FROM TEMPO.STATEMENT_LINE SUBPARTITION (EL_USA_SP1) sl
        WHERE sl.processing_office_id = 38
          AND sl.processing_office_id IN (1,2,5,6,7,16)
          AND sl.accounting_period_dt = DATE '2025-03-31'
          AND ROWNUM = 1
    ) THEN 'Y' ELSE 'N' END AS data_exists
FROM DUAL
UNION ALL
SELECT 
    '2025-04' AS month_period,
    CASE WHEN EXISTS (
        SELECT /*+ FIRST_ROWS(1) NO_PARALLEL */ 1 
        FROM TEMPO.STATEMENT_LINE SUBPARTITION (EL_USA_SP1) sl
        WHERE sl.processing_office_id = 38
          AND sl.processing_office_id IN (1,2,5,6,7,16)
          AND sl.accounting_period_dt = DATE '2025-04-30'
          AND ROWNUM = 1
    ) THEN 'Y' ELSE 'N' END AS data_exists
FROM DUAL
UNION ALL
SELECT 
    '2025-05' AS month_period,
    CASE WHEN EXISTS (
        SELECT /*+ FIRST_ROWS(1) NO_PARALLEL */ 1 
        FROM TEMPO.STATEMENT_LINE SUBPARTITION (EL_USA_SP1) sl
        WHERE sl.processing_office_id = 38
          AND sl.processing_office_id IN (1,2,5,6,7,16)
          AND sl.accounting_period_dt = DATE '2025-05-31'
          AND ROWNUM = 1
    ) THEN 'Y' ELSE 'N' END AS data_exists
FROM DUAL
UNION ALL
SELECT 
    '2025-06' AS month_period,
    CASE WHEN EXISTS (
        SELECT /*+ FIRST_ROWS(1) NO_PARALLEL */ 1 
        FROM TEMPO.STATEMENT_LINE SUBPARTITION (EL_USA_SP1) sl
        WHERE sl.processing_office_id = 38
          AND sl.processing_office_id IN (1,2,5,6,7,16)
          AND sl.accounting_period_dt = DATE '2025-06-30'
          AND ROWNUM = 1
    ) THEN 'Y' ELSE 'N' END AS data_exists
FROM DUAL
UNION ALL
SELECT 
    '2025-07' AS month_period,
    CASE WHEN EXISTS (
        SELECT /*+ FIRST_ROWS(1) NO_PARALLEL */ 1 
        FROM TEMPO.STATEMENT_LINE SUBPARTITION (EL_USA_SP1) sl
        WHERE sl.processing_office_id = 38
          AND sl.processing_office_id IN (1,2,5,6,7,16)
          AND sl.accounting_period_dt = DATE '2025-07-31'
          AND ROWNUM = 1
    ) THEN 'Y' ELSE 'N' END AS data_exists
FROM DUAL
UNION ALL
SELECT 
    '2025-08' AS month_period,
    CASE WHEN EXISTS (
        SELECT /*+ FIRST_ROWS(1) NO_PARALLEL */ 1 
        FROM TEMPO.STATEMENT_LINE SUBPARTITION (EL_USA_SP1) sl
        WHERE sl.processing_office_id = 38
          AND sl.processing_office_id IN (1,2,5,6,7,16)
          AND sl.accounting_period_dt = DATE '2025-08-31'
          AND ROWNUM = 1
    ) THEN 'Y' ELSE 'N' END AS data_exists
FROM DUAL
UNION ALL
SELECT 
    '2025-09' AS month_period,
    CASE WHEN EXISTS (
        SELECT /*+ FIRST_ROWS(1) NO_PARALLEL */ 1 
        FROM TEMPO.STATEMENT_LINE SUBPARTITION (EL_USA_SP1) sl
        WHERE sl.processing_office_id = 38
          AND sl.processing_office_id IN (1,2,5,6,7,16)
          AND sl.accounting_period_dt = DATE '2025-09-30'
          AND ROWNUM = 1
    ) THEN 'Y' ELSE 'N' END AS data_exists
FROM DUAL
UNION ALL
SELECT 
    '2025-10' AS month_period,
    CASE WHEN EXISTS (
        SELECT /*+ FIRST_ROWS(1) NO_PARALLEL */ 1 
        FROM TEMPO.STATEMENT_LINE SUBPARTITION (EL_USA_SP1) sl
        WHERE sl.processing_office_id = 38
          AND sl.processing_office_id IN (1,2,5,6,7,16)
          AND sl.accounting_period_dt = DATE '2025-10-31'
          AND ROWNUM = 1
    ) THEN 'Y' ELSE 'N' END AS data_exists
FROM DUAL
UNION ALL
SELECT 
    '2025-11' AS month_period,
    CASE WHEN EXISTS (
        SELECT /*+ FIRST_ROWS(1) NO_PARALLEL */ 1 
        FROM TEMPO.STATEMENT_LINE SUBPARTITION (EL_USA_SP1) sl
        WHERE sl.processing_office_id = 38
          AND sl.processing_office_id IN (1,2,5,6,7,16)
          AND sl.accounting_period_dt = DATE '2025-11-30'
          AND ROWNUM = 1
    ) THEN 'Y' ELSE 'N' END AS data_exists
FROM DUAL
UNION ALL
SELECT 
    '2025-12' AS month_period,
    CASE WHEN EXISTS (
        SELECT /*+ FIRST_ROWS(1) NO_PARALLEL */ 1 
        FROM TEMPO.STATEMENT_LINE SUBPARTITION (EL_USA_SP1) sl
        WHERE sl.processing_office_id = 38
          AND sl.processing_office_id IN (1,2,5,6,7,16)
          AND sl.accounting_period_dt = DATE '2025-12-31'
          AND ROWNUM = 1
    ) THEN 'Y' ELSE 'N' END AS data_exists
FROM DUAL
ORDER BY month_period;
