Музыка и видео в терминале: ASCII и цветные полублоки, CPU/CUDA/D3D11VA,
перемотка, пауза, громкость и повтор.

### Скачать и запустить

- **Windows 10/11 x64:** распакуйте `windows-x86_64.zip`, запустите `start.bat`.
  При первом запуске недостающий FFmpeg скачивается автоматически (~109 МБ).
- **Linux x64:** установите FFmpeg и ALSA (`sudo apt install ffmpeg libasound2`),
  распакуйте `linux-x86_64.tar.gz`, выполните `./start.sh` в терминале.
  Сборка рассчитана на glibc 2.35+ (Ubuntu 22.04+, Debian 12+).

Положите свои клипы или пары `Название.mp3` + `Название.mp4` в `media/`.
Rust и Python для готовых сборок не нужны. Громкость при запуске — 10%.

В этой версии меню очищается при обновлении и возврате из плеера;
сообщения об ошибках остаются видны. Пользовательских медиафайлов в архивах нет.

Windows and Linux x64 builds. Unpack the archive, add your files to `media/`,
then run `start.bat` or `./start.sh`. Windows downloads missing FFmpeg on first
launch; Linux requires system FFmpeg and ALSA. See README for controls and options.
