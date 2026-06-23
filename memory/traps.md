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

## Trap: CSS position:sticky sidebar jitter when scrolling to page bottom

### Context
App.vue dashboard-research sidebar, TopStatusBar.vue header

### Solution
Root cause: sticky element's margin-box overflows container's padding-box. Fix: (1) Pin header height exactly (e.g. height:3.5rem) so sticky top aligns without gap. (2) Set sticky element height to container-bottom minus 2px safety margin (calc(100vh - 3.5rem - 2rem - 2px)) to prevent push-out at scroll end. (3) Remove overflow-y:auto from sticky container itself — let internal child handle scroll.

## Trap: CSS position:sticky sidebar jitter when scrolling to page bottom

### Context
App.vue dashboard-research sidebar, TopStatusBar.vue header

### Solution
Root cause: sticky element's margin-box overflows container's padding-box. Fix: (1) Pin header height exactly (e.g. height:3.5rem) so sticky top aligns without gap. (2) Set sticky element height to container-bottom minus 2px safety margin (calc(100vh - 3.5rem - 2rem - 2px)) to prevent push-out at scroll end. (3) Remove overflow-y:auto from sticky container itself — let internal child handle scroll.
