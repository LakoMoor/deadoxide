<p align="center"><img src="assets/icon-256.png" width="128" alt="DeadOxide"></p>

# DeadOxide

Неофициальный совместимый рантайм **Project Zomboid** на Rust (по образцу [Horizon](https://github.com/horizon-rust/horizon) для Minecraft).
Читает файлы вашей **собственной** копии игры (Steam build 25485521, версия 42.21.0). Файлов игры в репозитории нет.

**Сайт с прогрессом и сравнением с оригиналом:** https://lakomoor.github.io/deadoxide/

Сейчас перенесено примерно 20 % (взвешенная оценка, часть областей — оценка автора; способ подсчёта на сайте и в `tools/gen_progress.mjs`).
Играть пока нельзя: нет зомби, инвентаря, звука и сети. Есть чтение форматов, запуск Lua игры и картинка мира.

## Запуск

```sh
git clone https://github.com/LakoMoor/deadoxide    # положите папку внутрь папки ProjectZomboid
cargo build --release
cargo run --release -p deadoxide-launcher           # лаунчер: играть, моды, прогресс (GPUI)
```

Клиент без лаунчера: `play.cmd`. Проверки: `sh tools/check.sh`.

## Моды

Каталог — `registry/index.json` (url, sha256). Лаунчер скачивает архив, сверяет sha256, распаковывает в `profile/mods`.
Добавить мод: пул-реквест с записью в каталоге; архив должен быть вашим или опубликован с разрешения автора.

## Устройство

См. [CLAUDE.md](CLAUDE.md), [docs/COMPATIBILITY.md](docs/COMPATIBILITY.md), [docs/PORT.md](docs/PORT.md).

Проект не связан с The Indie Stone. Скриншоты оригинальной игры в `screenshots/` и на сайте приложены как свидетельство сравнения; по просьбе правообладателя будут убраны.
