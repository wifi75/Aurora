; Installazione e disinstallazione forzate: se Aurora e' aperta (anche nascosta nel vassoio) la si chiude da soli,
; (solo il programma, non i suoi figli: l'installatore lanciato da Aurora non deve morire con lei) senza chiedere e senza fermarsi con «Aurora e' in esecuzione!». Il core riparte al prossimo avvio.

!macro AuroraStopRunning
  nsExec::Exec 'taskkill /F /IM aurora-desktop.exe'
  Pop $0
  Sleep 1200
!macroend

; Hamlib incluso (cartella "hamlib" accanto al programma): prima di copiare i file nuovi si ferma l'eventuale rigctld di Aurora rimasto vivo
; (solo quello lanciato da questa cartella, non i rigctld che l'utente usa per altri programmi) e si toglie la cartella vecchia,
; cosi' un aggiornamento non lascia file di versioni precedenti.
!macro AuroraCleanHamlib
  nsExec::Exec `powershell -NoProfile -WindowStyle Hidden -Command "Get-Process rigctl* -ErrorAction SilentlyContinue | Where-Object { $$_.Path -like '$INSTDIR\hamlib\*' } | Stop-Process -Force"`
  Pop $0
  Sleep 300
  RMDir /r "$INSTDIR\hamlib"
!macroend

!macro NSIS_HOOK_PREINSTALL
  !insertmacro AuroraStopRunning
  !insertmacro AuroraCleanHamlib
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro AuroraStopRunning
  !insertmacro AuroraCleanHamlib
!macroend
