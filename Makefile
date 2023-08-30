api_build:
	docker-compose up -d --build nginx db redis

up:
	docker-compose up -d nginx db redis

api_with_otel:
	docker-compose up -d