
## sqlx::query! 宏德编译时检查

需要在项目根目录中 `.env` 文件中配置环境变量

```
echo "DATABASE_URL=mysql://root:password@localhost:3306/polymarket" > .env
```

数据库表定义

```sql
CREATE TABLE `crypto_prices` (
  `id` int unsigned NOT NULL AUTO_INCREMENT,
  `symbol` char(8) COLLATE utf8mb4_general_ci NOT NULL,
  `price` decimal(38,0) NOT NULL,
  `timestamp` bigint NOT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB AUTO_INCREMENT=1975 DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci
```