; Installazione e disinstallazione forzate: se Aurora e' aperta (anche nascosta nel vassoio) la si chiude da soli,
; senza chiedere e senza fermarsi con «Aurora e' in esecuzione!». Il core riparte al prossimo avvio.

!macro AuroraStopRunning
  nsExec::Exec 'taskkill /F /T /IM aurora-desktop.exe'
  Pop $0
  Sleep 1200
!macroend

!macro NSIS_HOOK_PREINSTALL
  !insertmacro AuroraStopRunning
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro AuroraStopRunning
!macroend
