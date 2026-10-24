; Installazione e disinstallazione forzate: se Aurora e' aperta (anche nascosta nel vassoio) la si chiude da soli,
; (solo il programma, non i suoi figli: l'installatore lanciato da Aurora non deve morire con lei) senza chiedere e senza fermarsi con «Aurora e' in esecuzione!». Il core riparte al prossimo avvio.

!macro AuroraStopRunning
  nsExec::Exec 'taskkill /F /IM aurora-desktop.exe'
  Pop $0
  Sleep 1200
!macroend

!macro NSIS_HOOK_PREINSTALL
  !insertmacro AuroraStopRunning
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro AuroraStopRunning
!macroend
