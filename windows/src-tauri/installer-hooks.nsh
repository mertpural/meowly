; Meowly kurulum kancaları (Tauri NSIS şablonu çağırır).
; Kaldırırken: çalışan Meowly kapatılır; Windows ile başlat kaydı ve ~/.claude/settings.json içindeki
; yalnızca bu exe'yi çağıran durum satırı silinir. Kayıtlar ve ayarlar (%APPDATA%\Meowly) silinmez.
; Güncellemede ($UpdateMode = 1) dokunulmaz.
; Kurarken: adımlar %APPDATA%\Meowly\update.log'a yazılır (yalnızca sürüm ve sonuç). Güncellemede dosyalar
; kopyalanmadan önce açık kalan Meowly süreçleri kapatılır: exe kilitli kalırsa kurulum eski sürümü bırakıyordu.

!macro MEOWLY_LOG text
  CreateDirectory "$APPDATA\Meowly"
  ClearErrors
  FileOpen $9 "$APPDATA\Meowly\update.log" a
  ${IfNot} ${Errors}
    FileSeek $9 0 END
    FileWrite $9 "kurulum: ${text}$\r$\n"
    FileClose $9
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREINSTALL
  !insertmacro MEOWLY_LOG "${VERSION} basladi (guncelleme=$UpdateMode, sessiz=$PassiveMode, klasor=$INSTDIR)"
  ${If} $UpdateMode = 1
    nsExec::Exec 'taskkill /F /IM meowly.exe'
    Pop $0
    !insertmacro MEOWLY_LOG "acik Meowly surecleri kapatildi (taskkill=$0)"
    Sleep 1000
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ClearErrors
  GetDLLVersion "$INSTDIR\meowly.exe" $R8 $R9
  ${If} ${Errors}
    !insertmacro MEOWLY_LOG "${VERSION} bitti, kurulan exe surumu okunamadi"
  ${Else}
    IntOp $R7 $R8 >> 16
    IntOp $R8 $R8 & 0xFFFF
    IntOp $R6 $R9 >> 16
    !insertmacro MEOWLY_LOG "${VERSION} bitti, kurulan exe: $R7.$R8.$R6"
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ${If} $UpdateMode <> 1
    ExecWait '"$INSTDIR\meowly.exe" --uninstall-cleanup'
  ${EndIf}
!macroend
