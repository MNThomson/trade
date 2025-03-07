CREATE TABLE users (
    user_id BIGSERIAL PRIMARY KEY,
    user_name TEXT NOT NULL UNIQUE,
    password TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE stocks (
    stock_id BIGSERIAL PRIMARY KEY,
    stock_name TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE orders (
    order_id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    stock_id BIGINT NOT NULL,
    amount BIGINT NOT NULL,
    limit_price BIGINT,
    order_status BIGINT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(user_id),
    FOREIGN KEY (stock_id) REFERENCES stocks(stock_id)
);

CREATE TABLE trades (
    trade_id BIGSERIAL PRIMARY KEY,
    sell_order BIGINT NOT NULL,
    buy_order BIGINT NOT NULL,
    amount BIGINT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (sell_order) REFERENCES orders(order_id),
    FOREIGN KEY (buy_order) REFERENCES orders(order_id)
);

CREATE TABLE deposits (
    deposit_id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    amount BIGINT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(user_id)
);

INSERT INTO users (user_name, password) VALUES
('admin', '$argon2id$v=19$m=1024,t=1,p=1$HAZcjX8wBnPhvVhYBpXO5g$H009UoKExbLzSHbl5Ru6WEQ4djyRi5sU8fkfCwk8ulI');
