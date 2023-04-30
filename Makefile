dev:
	sqlx db create
	sqlx migrate run
	cargo wathc -x run
db:
	docker-compose up -d
test:
	cargo test
watch:
	cargo watch -x run