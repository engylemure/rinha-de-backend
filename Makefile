api_build:
	docker-compose up -d --build nginx

up:
	docker-compose up -d nginx
api_with_otel:
	docker-compose up -d