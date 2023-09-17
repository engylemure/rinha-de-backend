# Default configuration with STDOUT log output
dc-up:
	docker-compose up -d nginx

dc-down: 
	docker-compose down

dc-build:
	docker-compose up -d --build nginx

dc-only-build:
	docker-compose build

dc-restart: dc-down dc-up

# Host network is only available on linux hosts see https://docs.docker.com/network/drivers/host/
dc-up-host_net:
	docker-compose -f docker-compose.host-network.yml up -d nginx

dc-down-host_net:
	docker-compose -f docker-compose.host-network.yml down

dc-restart-host_net: dc-down-host_net dc-up-host_net

# API with Open Telemetry enabled with Jaeger and Prometheus to compute real time metrics and trace
dc-up-otel:
	docker-compose -f docker-compose.otel.yml up -d nginx

dc-down-otel:
	docker-compose -f docker-compose.otel.yml down

dc-restart-otel: dc-down-otel dc-up-otel


dc-up-no-cache:
	docker-compose -f docker-compose.without-cache.yml up -d nginx

dc-down-no-cache:
	docker-compose -f docker-compose.without-cache.yml down

dc-restart-no-cache: dc-down-no-cache dc-up-no-cache