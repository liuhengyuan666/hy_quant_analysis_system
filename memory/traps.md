## Trap: 

### Context


### Solution


## Trap: 

### Context


### Solution


## Trap: 

### Context


### Solution


## Trap: 

### Context


### Root Cause
ClickHouse default max_partitions_per_insert_block=100 is too low for long-history symbols spanning multiple years

### Solution


### Prevention
All new ClickHouse INSERT queries must include partition limit settings when handling multi-year historical data

## Trap: 

### Context


### Root Cause
fetch_tencent_daily_bars had hardcoded count=400 in the API request, but Tencent actually supports up to 1000 rows per request

### Solution


### Prevention
All multi-year data fetchers must implement automatic pagination; never hardcode provider row limits
