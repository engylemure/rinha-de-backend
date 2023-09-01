# rinha de backend

A implementation for the challenge [rinha de backend](https://github.com/zanfranceschi/rinha-de-backend-2023-q3), inspired(parts of it are copy pasted) from [rinha-backend-rust](https://github.com/viniciusfonseca/rinha-backend-rust)

### Current local Results for my Macbook M1 16gb
 - Without Tracing: 46576
 ![local results for the implementation with cors and without tracing](./without_tracing.png)
 - With StdOut Tracing: 46576
 ![local results for the implementation with cors and stdout tracing](./with_stdout_tracing.png)
 - With Open Telemetry Tracing: 46576
 ![local results for the implementation with cors and open telemetry tracing](./with_otel_tracing.png)
 - Winner from the challenge: 47089 this value is slightly wrong sine the total amount of requests for insertion is 46576
 ![local results for the implementation from viniciusfonseca the winner from the challenge](./challenge_winner.png)