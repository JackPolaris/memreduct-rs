; Polish installer text for Mem Reduct.
;
; Tauri's NSIS bundle ships translations for 21 languages but not Polish (nor
; Vietnamese/Thai/Indonesian). Listing a language in `bundle > windows > nsis >
; languages` without a matching file here is NOT an error — Tauri only logs a
; warning and silently drops the language from the installer, which is why
; these files exist.
;
; Keys, placeholders (`{{product_name}}`, `${PRODUCTNAME}`, `${VERSION}`, `$0`,
; `$1`, `$R4`, `$\n`) and the `choowHowToInstall` typo must match Tauri's
; English.nsh exactly — it is the contract, not a suggestion.
;
; Source of truth:
; https://github.com/tauri-apps/tauri/blob/dev/crates/tauri-bundler/src/bundle/windows/nsis/languages/English.nsh

LangString addOrReinstall ${LANG_POLISH} "Dodaj / zainstaluj ponownie składniki"
LangString alreadyInstalled ${LANG_POLISH} "Już zainstalowano"
LangString alreadyInstalledLong ${LANG_POLISH} "${PRODUCTNAME} ${VERSION} jest już zainstalowany. Wybierz operację, którą chcesz wykonać, i kliknij Dalej, aby kontynuować."
LangString appRunning ${LANG_POLISH} "{{product_name}} jest uruchomiony! Zamknij go i spróbuj ponownie."
LangString appRunningOkKill ${LANG_POLISH} "{{product_name}} jest uruchomiony!$\nKliknij OK, aby go zamknąć"
LangString chooseMaintenanceOption ${LANG_POLISH} "Wybierz operację konserwacyjną do wykonania."
LangString choowHowToInstall ${LANG_POLISH} "Wybierz sposób instalacji ${PRODUCTNAME}."
LangString createDesktop ${LANG_POLISH} "Utwórz skrót na pulpicie"
LangString dontUninstall ${LANG_POLISH} "Nie odinstalowuj"
LangString dontUninstallDowngrade ${LANG_POLISH} "Nie odinstalowuj (ten instalator nie pozwala zainstalować starszej wersji bez odinstalowania)"
LangString failedToKillApp ${LANG_POLISH} "Nie udało się zamknąć {{product_name}}. Zamknij go ręcznie i spróbuj ponownie"
LangString installingWebview2 ${LANG_POLISH} "Instalowanie WebView2..."
LangString newerVersionInstalled ${LANG_POLISH} "Nowsza wersja ${PRODUCTNAME} jest już zainstalowana! Instalowanie starszej wersji nie jest zalecane. Jeśli naprawdę chcesz zainstalować tę starszą wersję, lepiej najpierw odinstalować obecną. Wybierz operację, którą chcesz wykonać, i kliknij Dalej, aby kontynuować."
LangString older ${LANG_POLISH} "starsza"
LangString olderOrUnknownVersionInstalled ${LANG_POLISH} "W systemie jest zainstalowana wersja $R4 programu ${PRODUCTNAME}. Zaleca się odinstalowanie obecnej wersji przed instalacją. Wybierz operację, którą chcesz wykonać, i kliknij Dalej, aby kontynuować."
LangString silentDowngrades ${LANG_POLISH} "Ten instalator nie pozwala zainstalować starszej wersji, więc nie można kontynuować w trybie cichym. Użyj instalatora z interfejsem graficznym.$\n"
LangString unableToUninstall ${LANG_POLISH} "Nie można odinstalować!"
LangString uninstallApp ${LANG_POLISH} "Odinstaluj ${PRODUCTNAME}"
LangString uninstallBeforeInstalling ${LANG_POLISH} "Odinstaluj przed instalacją"
LangString unknown ${LANG_POLISH} "nieznana"
LangString webview2AbortError ${LANG_POLISH} "Nie udało się zainstalować WebView2! Aplikacja nie może działać bez niego. Spróbuj ponownie uruchomić instalator."
LangString webview2DownloadError ${LANG_POLISH} "Błąd: pobieranie WebView2 nie powiodło się - $0"
LangString webview2DownloadSuccess ${LANG_POLISH} "Pomyślnie pobrano program instalacyjny WebView2"
LangString webview2Downloading ${LANG_POLISH} "Pobieranie programu instalacyjnego WebView2..."
LangString webview2InstallError ${LANG_POLISH} "Błąd: instalacja WebView2 nie powiodła się, kod wyjścia $1"
LangString webview2InstallSuccess ${LANG_POLISH} "Pomyślnie zainstalowano WebView2"
LangString deleteAppData ${LANG_POLISH} "Usuń dane aplikacji"
