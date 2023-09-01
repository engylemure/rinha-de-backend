# rinha de backend

A implementation for the challenge [rinha de backend](https://github.com/zanfranceschi/rinha-de-backend-2023-q3), inspired(parts of it are copy pasted) from [rinha-backend-rust](https://github.com/viniciusfonseca/rinha-backend-rust)

### Current local Results for a Ryzen 7 32gb ddr4 and NVME 7000r/6000w mb/s with the Docker compose restrictions
 - Without Cors and Tracing: 46576
 ![local results for the implementation with cors and tracing disabled](./without_tracing.html.png)
 - With Cors and Tracing: 46576
 ![local results for the implementation with cors and tracing enabled](./with_tracing.html.png)
 - Winner from the challenge: 47089 this value is slightly wrong sine the total amount of requests for insertion is 46576
 ![local results for the implementation from viniciusfonseca the winner from the challenge](./challenge_winner.png)