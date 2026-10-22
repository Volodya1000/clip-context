# Установка clip-context без Rust

Готовые пакеты для Windows x86_64 и Linux x86_64 собираются
в [GitHub Actions](../.github/workflows/release.yml). Pull request только
проверяет сборки. **Пока не опубликован первый релиз с тегом, загрузить
готовые пакеты нельзя.** [Страница релизов](https://github.com/Volodya1000/clip-context/releases).

## Windows (PowerShell)

Скачайте и изучите [скрипт установки](../scripts/install.ps1), затем запустите
его из папки с репозиторием:

```powershell
powershell -NoProfile -File .\scripts\install.ps1
cpctx --version
cpctx --help
```

Скрипт загружает последний стабильный ZIP, проверяет SHA-256 и версию исполняемого
файла, устанавливает `cpctx.exe` в
`%LOCALAPPDATA%\Programs\clip-context` и добавляет каталог в **пользовательский
PATH**. Права администратора и Rust не нужны. Откройте новый терминал, если
ранее запущенная оболочка не увидела изменение. Указав `-Version vX.Y.Z`,
можно закрепить версию. Для удаления:

```powershell
powershell -NoProfile -File .\scripts\install.ps1 -Uninstall
```

## Ubuntu, Debian, Linux Mint (x86_64)

Скачайте файл `clip-context_X.Y.Z_amd64.deb` из опубликованного релиза:

```sh
sudo dpkg -i clip-context_X.Y.Z_amd64.deb
cpctx --version
cpctx --help
```

Исполняемый файл окажется в `/usr/bin/cpctx`, обычно уже входящем в `PATH`.
Удаление: `sudo dpkg -r clip-context`. Пакет собирается на Ubuntu 22.04;
не заявляем совместимость со старыми glibc, Alpine/musl, ARM64 или macOS.

## Другие Linux

Скачайте архив `cpctx-vX.Y.Z-linux-x86_64.tar.gz` и `SHA256SUMS.txt`,
проверьте контрольную сумму соответствующего файла, распакуйте и установите:

```sh
tar -xzf cpctx-vX.Y.Z-linux-x86_64.tar.gz
sudo install -m 755 cpctx /usr/local/bin/cpctx
cpctx --version
```

`/usr/local/bin` обычно входит в `PATH`.

## Для выпускающего релиз

Сначала пройдут CI и ручная приёмка на обеих системах. После этого создаётся
тег вида `vX.Y.Z`, указывающий на коммит `main` и совпадающий с
`Cargo.toml`. Только запуск по тегу публикует настоящие активы релиза;
запуск по pull request или вручную только собирает и проверяет пакеты.
