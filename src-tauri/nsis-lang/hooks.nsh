; Mem Reduct — NSIS installer hooks.
; Wired up via `bundle > windows > nsis > installerHooks` and `!include`d near
; the top of Tauri's generated installer.nsi, so the macro below is only
; expanded when `!insertmacro` runs it further down.
;
; WHY THIS FILE EXISTS
; --------------------
; Tauri's installer.nsi declares where the language picker should remember its
; choice:
;
;     !define MUI_LANGDLL_REGISTRY_ROOT     "HKCU"
;     !define MUI_LANGDLL_REGISTRY_KEY      "${MANUPRODUCTKEY}"
;     !define MUI_LANGDLL_REGISTRY_VALUENAME "Installer Language"
;
; and NSIS' MUI_LANGDLL_DISPLAY *reads* that value before deciding whether to
; show the dialog. But the template never inserts MUI_LANGDLL_SAVELANGUAGE —
; the one and only macro that *writes* the value. So it stays empty forever,
; which means:
;
;   * the language selector never remembers anything, even between two
;     consecutive runs of the same installer; and
;   * worse, the dialog reappears on every single run. It is skipped only when
;     `${Silent}` is set (i.e. `/S`), and the auto-updater invokes the installer
;     with `/P` (passive) — so a modal language dialog would pop up in the
;     middle of every background update, and the user dismissing it hits
;     `Abort`, which aborts the whole install.
;
; Writing the value here fixes both. Note `${MANUPRODUCTKEY}` is `!define`d
; *after* this file is included, which is fine: NSIS substitutes it when the
; macro is inserted, not when it is defined.
;
; The value is an NSIS language id — a Windows LCID in decimal. The app reads it
; on first launch only; see `src-tauri/src/installer_lang.rs`.

!macro NSIS_HOOK_POSTINSTALL
  ; Unconditional on purpose: re-running the GUI installer with a different
  ; language should update the remembered choice. During an auto-update
  ; $LANGUAGE was seeded from this very value, so writing it back is a no-op.
  WriteRegStr HKCU "${MANUPRODUCTKEY}" "Installer Language" $LANGUAGE
!macroend
