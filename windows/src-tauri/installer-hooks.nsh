; Meowly kurulum kancaları (Tauri NSIS şablonu çağırır).
; Kaldırırken: çalışan Meowly kapatılır; Windows ile başlat kaydı ve ~/.claude/settings.json içindeki
; yalnızca bu exe'yi çağıran durum satırı silinir. Kayıtlar ve ayarlar (%APPDATA%\Meowly) silinmez.
; Güncellemede ($UpdateMode = 1) dokunulmaz.

!macro NSIS_HOOK_PREUNINSTALL
  ${If} $UpdateMode <> 1
    ExecWait '"$INSTDIR\meowly.exe" --uninstall-cleanup'
  ${EndIf}
!macroend