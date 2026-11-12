; Modello NSIS dell'installatore di Aurora: e' quello di Tauri (tauri-bundler, @tauri-apps/cli 2.12.4, licenza MIT/Apache-2.0) con le
; modifiche segnate «AURORA»: con una versione piu' vecchia gia' installata la scelta di base e' l'aggiornamento, la disinstallazione
; non cancella mai i dati (niente casella «Elimina i dati»), la pagina dei dati che escono dal PC e il registro dell'installazione con il
; rapporto quando non riesce (funzioni in hooks.nsh). Se si aggiorna @tauri-apps/cli, ripartire dal suo modello nuovo e rifare le modifiche.
Unicode true
ManifestDPIAware true
; Add in `dpiAwareness` `PerMonitorV2` to manifest for Windows 10 1607+ (note this should not affect lower versions since they should be able to ignore this and pick up `dpiAware` `true` set by `ManifestDPIAware true`)
; Currently undocumented on NSIS's website but is in the Docs folder of source tree, see
; https://github.com/kichik/nsis/blob/5fc0b87b819a9eec006df4967d08e522ddd651c9/Docs/src/attributes.but#L286-L300
; https://github.com/tauri-apps/tauri/pull/10106
ManifestDPIAwareness PerMonitorV2

!if "{{compression}}" == "none"
  SetCompress off
!else
  ; Set the compression algorithm. We default to LZMA.
  SetCompressor /SOLID "{{compression}}"
!endif

; Keep above !include to stay ahead of any plugin command
; see https://github.com/tauri-apps/tauri/pull/15422#discussion_r3289239624
{{#if signed_plugins_path}}
!addplugindir "{{signed_plugins_path}}"
{{/if}}

!include MUI2.nsh
!include FileFunc.nsh
!include x64.nsh
!include WordFunc.nsh
!include "utils.nsh"
!include "FileAssociation.nsh"
!include "Win\COM.nsh"
!include "Win\Propkey.nsh"
!include "Win\RestartManager.nsh"
!include "StrFunc.nsh"
${StrCase}
${StrLoc}

{{#if installer_hooks}}
!include "{{installer_hooks}}"
{{/if}}

!define WEBVIEW2APPGUID "{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"

!define MANUFACTURER "{{manufacturer}}"
!define PRODUCTNAME "{{product_name}}"
!define VERSION "{{version}}"
!define VERSIONWITHBUILD "{{version_with_build}}"
!define HOMEPAGE "{{homepage}}"
!define INSTALLMODE "{{install_mode}}"
!define LICENSE "{{license}}"
!define INSTALLERICON "{{installer_icon}}"
!define SIDEBARIMAGE "{{sidebar_image}}"
!define HEADERIMAGE "{{header_image}}"
!define UNINSTALLERICON "{{uninstaller_icon}}"
!define UNINSTALLERHEADERIMAGE "{{uninstaller_header_image}}"
!define MAINBINARYNAME "{{main_binary_name}}"
!define MAINBINARYSRCPATH "{{main_binary_path}}"
!define BUNDLEID "{{bundle_id}}"
!define COPYRIGHT "{{copyright}}"
!define OUTFILE "{{out_file}}"
!define ARCH "{{arch}}"
!define ADDITIONALPLUGINSPATH "{{additional_plugins_path}}"
!define ALLOWDOWNGRADES "{{allow_downgrades}}"
!define DISPLAYLANGUAGESELECTOR "{{display_language_selector}}"
!define INSTALLWEBVIEW2MODE "{{install_webview2_mode}}"
!define WEBVIEW2INSTALLERARGS "{{webview2_installer_args}}"
!define WEBVIEW2BOOTSTRAPPERPATH "{{webview2_bootstrapper_path}}"
!define WEBVIEW2INSTALLERPATH "{{webview2_installer_path}}"
!define MINIMUMWEBVIEW2VERSION "{{minimum_webview2_version}}"
!define UNINSTKEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCTNAME}"
!define MANUKEY "Software\${MANUFACTURER}"
!define MANUPRODUCTKEY "${MANUKEY}\${PRODUCTNAME}"
!define UNINSTALLERSIGNCOMMAND "{{uninstaller_sign_cmd}}"
!define ESTIMATEDSIZE "{{estimated_size}}"
!define STARTMENUFOLDER "{{start_menu_folder}}"

Var AuroraTop
Var AuroraLow
Var PassiveMode
Var UpdateMode
Var NoShortcutMode
Var WixMode
Var OldMainBinaryName

Name "${PRODUCTNAME}"
BrandingText "${COPYRIGHT}"
OutFile "${OUTFILE}"

; We don't actually use this value as default install path,
; it's just for nsis to append the product name folder in the directory selector
; https://nsis.sourceforge.io/Reference/InstallDir
!define PLACEHOLDER_INSTALL_DIR "placeholder\${PRODUCTNAME}"
InstallDir "${PLACEHOLDER_INSTALL_DIR}"

VIProductVersion "${VERSIONWITHBUILD}"
VIAddVersionKey "ProductName" "${PRODUCTNAME}"
VIAddVersionKey "FileDescription" "${PRODUCTNAME}"
VIAddVersionKey "LegalCopyright" "${COPYRIGHT}"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "ProductVersion" "${VERSION}"

# additional plugins
!addplugindir "${ADDITIONALPLUGINSPATH}"

; Uninstaller signing command
!if "${UNINSTALLERSIGNCOMMAND}" != ""
  !uninstfinalize '${UNINSTALLERSIGNCOMMAND}'
!endif

; Handle install mode, `perUser`, `perMachine` or `both`
!if "${INSTALLMODE}" == "perMachine"
  RequestExecutionLevel admin
!endif

!if "${INSTALLMODE}" == "currentUser"
  RequestExecutionLevel user
!endif

!if "${INSTALLMODE}" == "both"
  !define MULTIUSER_MUI
  !define MULTIUSER_INSTALLMODE_INSTDIR "${PRODUCTNAME}"
  !define MULTIUSER_INSTALLMODE_COMMANDLINE
  !if "${ARCH}" == "x64"
    !define MULTIUSER_USE_PROGRAMFILES64
  !else if "${ARCH}" == "arm64"
    !define MULTIUSER_USE_PROGRAMFILES64
  !endif
  !define MULTIUSER_INSTALLMODE_DEFAULT_REGISTRY_KEY "${UNINSTKEY}"
  !define MULTIUSER_INSTALLMODE_DEFAULT_REGISTRY_VALUENAME "CurrentUser"
  !define MULTIUSER_INSTALLMODEPAGE_SHOWUSERNAME
  !define MULTIUSER_INSTALLMODE_FUNCTION RestorePreviousInstallLocation
  !define MULTIUSER_EXECUTIONLEVEL Highest
  !include MultiUser.nsh
!endif

; Installer icon
!if "${INSTALLERICON}" != ""
  !define MUI_ICON "${INSTALLERICON}"
!endif

; Installer sidebar image
!if "${SIDEBARIMAGE}" != ""
  !define MUI_WELCOMEFINISHPAGE_BITMAP "${SIDEBARIMAGE}"
!endif

; Enable header images for installer and uninstaller pages when either image is configured.
!if "${HEADERIMAGE}" != ""
  !define MUI_HEADERIMAGE
!else if "${UNINSTALLERHEADERIMAGE}" != ""
  !define MUI_HEADERIMAGE
!endif

; Installer header image
!if "${HEADERIMAGE}" != ""
  !define MUI_HEADERIMAGE_BITMAP "${HEADERIMAGE}"
!endif
; AURORA: logo a destra, cosi' titolo e sottotitolo della testata partono dal bordo sinistro come il resto della pagina
!define MUI_HEADERIMAGE_RIGHT

; Uninstaller header image
!if "${UNINSTALLERHEADERIMAGE}" != ""
  !define MUI_HEADERIMAGE_UNBITMAP "${UNINSTALLERHEADERIMAGE}"
!endif

; Uninstaller icon
!if "${UNINSTALLERICON}" != ""
  !define MUI_UNICON "${UNINSTALLERICON}"
!endif

; Define registry key to store installer language
!define MUI_LANGDLL_REGISTRY_ROOT "HKCU"
!define MUI_LANGDLL_REGISTRY_KEY "${MANUPRODUCTKEY}"
!define MUI_LANGDLL_REGISTRY_VALUENAME "Installer Language"

; Installer pages, must be ordered as they appear
; 1. Welcome Page
!define MUI_PAGE_CUSTOMFUNCTION_PRE SkipIfPassive
!insertmacro MUI_PAGE_WELCOME

; 2. License Page (if defined)
!if "${LICENSE}" != ""
  !define MUI_PAGE_CUSTOMFUNCTION_PRE SkipIfPassive
  !insertmacro MUI_PAGE_LICENSE "${LICENSE}"
!endif

; 3. Install mode (if it is set to `both`)
!if "${INSTALLMODE}" == "both"
  !define MUI_PAGE_CUSTOMFUNCTION_PRE SkipIfPassive
  !insertmacro MULTIUSER_PAGE_INSTALLMODE
!endif

; 4. Custom page to ask user if he wants to reinstall/uninstall
;    only if a previous installation was detected
Var ReinstallPageCheck
Page custom PageReinstall PageLeaveReinstall

; AURORA: 5. Dati che escono dal PC (richiesto dalla firma gratuita di SignPath: dirlo durante l'installazione, con il modo di spegnerli)
Page custom PageDatiAurora
Function PageDatiAurora
  ${IfThen} $PassiveMode = 1 ${|} Abort ${|}
  nsDialogs::Create 1018
  Pop $0
  ${If} $LANGUAGE = 1040
    ${NSD_CreateLabel} 0 0 100% 12u "Dati che Aurora manda in rete"
    Pop $1
    ${NSD_CreateLabel} 0 14u 100% 12u "Aurora tiene log, impostazioni e password sul tuo PC. Escono solo questi dati:"
    Pop $1
    ${NSD_CreateLabel} 0 28u 100% 28u "- rapporti degli errori anonimi (versione, radio, scheda audio, messaggio d'errore ripulito; mai nominativo, locatore, log o password) e conteggio delle installazioni attive con un codice casuale;"
    Pop $1
    ${NSD_CreateLabel} 0 58u 100% 20u "- PSK Reporter (spot ricevuti, con nominativo e locatore), DX Cluster, controllo degli aggiornamenti su GitHub, propagazione e messaggi dell'autore;"
    Pop $1
    ${NSD_CreateLabel} 0 80u 100% 12u "- QRZ.com, HamQTH, eQSL, LoTW, ClubLog, WSPRnet e gli altri servizi solo se li accendi tu."
    Pop $1
    ${NSD_CreateLabel} 0 94u 100% 28u "Si spengono in qualunque momento in Impostazioni (Assistenza > Rapporti degli errori, Servizi online, Programmi collegati). Dettagli: docs/privacy.md su github.com/wifi75/Aurora."
    Pop $1
  ${ElseIf} $LANGUAGE = 1031
    ${NSD_CreateLabel} 0 0u 100% 12u "Daten, die Aurora ins Netz sendet"
    Pop $1
    ${NSD_CreateLabel} 0 14u 100% 20u "Aurora speichert Log, Einstellungen und Passwörter auf Ihrem PC. Nur diese Daten verlassen ihn:"
    Pop $1
    ${NSD_CreateLabel} 0 36u 100% 34u "- anonyme Fehlerberichte (Version, Funkgerät, Soundkarte, bereinigte Fehlermeldung; nie Rufzeichen, Locator, Log oder Passwörter) und eine Zählung der aktiven Installationen mit einem Zufallscode;"
    Pop $1
    ${NSD_CreateLabel} 0 72u 100% 24u "- PSK Reporter (empfangene Spots, mit Rufzeichen und Locator), DX-Cluster, Update-Prüfung auf GitHub, Ausbreitung und Nachrichten des Autors;"
    Pop $1
    ${NSD_CreateLabel} 0 98u 100% 12u "- QRZ.com, HamQTH, eQSL, LoTW, ClubLog, WSPRnet und die anderen Dienste nur, wenn Sie sie einschalten."
    Pop $1
    ${NSD_CreateLabel} 0 112u 100% 28u "Sie lassen sich jederzeit in den Einstellungen ausschalten (Support > Fehlerberichte, Online-Dienste, Verbundene Programme). Details: docs/privacy.md auf github.com/wifi75/Aurora."
    Pop $1
  ${ElseIf} $LANGUAGE = 1036
    ${NSD_CreateLabel} 0 0u 100% 12u "Données qu'Aurora envoie sur le réseau"
    Pop $1
    ${NSD_CreateLabel} 0 14u 100% 20u "Aurora garde journal, réglages et mots de passe sur votre PC. Seules ces données en sortent :"
    Pop $1
    ${NSD_CreateLabel} 0 36u 100% 34u "- rapports d'erreurs anonymes (version, radio, carte son, message d'erreur nettoyé ; jamais indicatif, locator, journal ni mots de passe) et comptage des installations actives avec un code aléatoire ;"
    Pop $1
    ${NSD_CreateLabel} 0 72u 100% 24u "- PSK Reporter (spots reçus, avec indicatif et locator), DX Cluster, vérification des mises à jour sur GitHub, propagation et messages de l'auteur ;"
    Pop $1
    ${NSD_CreateLabel} 0 98u 100% 12u "- QRZ.com, HamQTH, eQSL, LoTW, ClubLog, WSPRnet et les autres services seulement si vous les activez."
    Pop $1
    ${NSD_CreateLabel} 0 112u 100% 28u "Vous pouvez les désactiver à tout moment dans les Réglages (Assistance > Rapports d'erreurs, Services en ligne, Programmes connectés). Détails : docs/privacy.md sur github.com/wifi75/Aurora."
    Pop $1
  ${ElseIf} $LANGUAGE = 1034
    ${NSD_CreateLabel} 0 0u 100% 12u "Datos que Aurora envía a la red"
    Pop $1
    ${NSD_CreateLabel} 0 14u 100% 20u "Aurora guarda el registro, los ajustes y las contraseñas en tu PC. Solo salen estos datos:"
    Pop $1
    ${NSD_CreateLabel} 0 36u 100% 34u "- informes de errores anónimos (versión, radio, tarjeta de sonido, mensaje de error depurado; nunca indicativo, locator, registro ni contraseñas) y recuento de instalaciones activas con un código aleatorio;"
    Pop $1
    ${NSD_CreateLabel} 0 72u 100% 24u "- PSK Reporter (spots recibidos, con indicativo y locator), DX Cluster, comprobación de actualizaciones en GitHub, propagación y mensajes del autor;"
    Pop $1
    ${NSD_CreateLabel} 0 98u 100% 12u "- QRZ.com, HamQTH, eQSL, LoTW, ClubLog, WSPRnet y los demás servicios solo si los activas tú."
    Pop $1
    ${NSD_CreateLabel} 0 112u 100% 28u "Se desactivan en cualquier momento en Ajustes (Asistencia > Informes de errores, Servicios en línea, Programas conectados). Detalles: docs/privacy.md en github.com/wifi75/Aurora."
    Pop $1
  ${ElseIf} $LANGUAGE = 2070
    ${NSD_CreateLabel} 0 0u 100% 12u "Dados que o Aurora envia para a rede"
    Pop $1
    ${NSD_CreateLabel} 0 14u 100% 20u "O Aurora guarda o registo, as definições e as palavras-passe no seu PC. Só estes dados saem dele:"
    Pop $1
    ${NSD_CreateLabel} 0 36u 100% 34u "- relatórios de erros anónimos (versão, rádio, placa de som, mensagem de erro limpa; nunca indicativo, locator, registo nem palavras-passe) e contagem das instalações ativas com um código aleatório;"
    Pop $1
    ${NSD_CreateLabel} 0 72u 100% 24u "- PSK Reporter (spots recebidos, com indicativo e locator), DX Cluster, verificação de atualizações no GitHub, propagação e mensagens do autor;"
    Pop $1
    ${NSD_CreateLabel} 0 98u 100% 12u "- QRZ.com, HamQTH, eQSL, LoTW, ClubLog, WSPRnet e os outros serviços só se os ativar."
    Pop $1
    ${NSD_CreateLabel} 0 112u 100% 28u "Pode desativá-los a qualquer momento em Definições (Assistência > Relatórios de erros, Serviços online, Programas ligados). Detalhes: docs/privacy.md em github.com/wifi75/Aurora."
    Pop $1
  ${ElseIf} $LANGUAGE = 1043
    ${NSD_CreateLabel} 0 0u 100% 12u "Gegevens die Aurora het netwerk op stuurt"
    Pop $1
    ${NSD_CreateLabel} 0 14u 100% 20u "Aurora bewaart logboek, instellingen en wachtwoorden op je pc. Alleen deze gegevens verlaten die:"
    Pop $1
    ${NSD_CreateLabel} 0 36u 100% 34u "- anonieme foutrapporten (versie, radio, geluidskaart, opgeschoond foutbericht; nooit roepnaam, locator, logboek of wachtwoorden) en een telling van actieve installaties met een willekeurige code;"
    Pop $1
    ${NSD_CreateLabel} 0 72u 100% 24u "- PSK Reporter (ontvangen spots, met roepnaam en locator), DX Cluster, controle op updates op GitHub, propagatie en berichten van de auteur;"
    Pop $1
    ${NSD_CreateLabel} 0 98u 100% 12u "- QRZ.com, HamQTH, eQSL, LoTW, ClubLog, WSPRnet en de andere diensten alleen als jij ze inschakelt."
    Pop $1
    ${NSD_CreateLabel} 0 112u 100% 28u "Je schakelt ze op elk moment uit in Instellingen (Ondersteuning > Foutrapporten, Onlinediensten, Gekoppelde programma's). Details: docs/privacy.md op github.com/wifi75/Aurora."
    Pop $1
  ${Else}
    ${NSD_CreateLabel} 0 0 100% 12u "Data Aurora sends over the network"
    Pop $1
    ${NSD_CreateLabel} 0 14u 100% 12u "Aurora keeps your log, settings and passwords on your PC. Only this data leaves it:"
    Pop $1
    ${NSD_CreateLabel} 0 28u 100% 28u "- anonymous error reports (version, radio, sound card, cleaned error message; never callsign, locator, log or passwords) and a count of active installations with a random code;"
    Pop $1
    ${NSD_CreateLabel} 0 58u 100% 20u "- PSK Reporter (received spots, with callsign and locator), DX Cluster, update check on GitHub, propagation and messages from the author;"
    Pop $1
    ${NSD_CreateLabel} 0 80u 100% 12u "- QRZ.com, HamQTH, eQSL, LoTW, ClubLog, WSPRnet and the other services only if you turn them on."
    Pop $1
    ${NSD_CreateLabel} 0 94u 100% 28u "You can turn them off at any time in Settings (Support > Error reports, Online services, Connected programs). Details: docs/privacy.md at github.com/wifi75/Aurora."
    Pop $1
  ${EndIf}
  nsDialogs::Show
FunctionEnd

; Find a previous WiX installation and set $R6 to its uninstall registry key, or to "" if none.
;
; A WiX installer stores the installation info in registry
; using a UUID and so we have to loop through all keys under
; `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall`
; and check if `DisplayName` and `Publisher` keys match ${PRODUCTNAME} and ${MANUFACTURER}
;
; This has a potential issue that there maybe another installation that matches
; our ${PRODUCTNAME} and ${MANUFACTURER} but wasn't installed by our WiX installer,
; however, this should be fine since the user will have to confirm the uninstallation
; and they can chose to abort it if doesn't make sense.
Function FindWixInstall
  StrCpy $R6 ""
  StrCpy $0 0
  wix_loop:
    EnumRegKey $1 HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall" $0
    StrCmp $1 "" wix_loop_done ; Exit loop if there is no more keys to loop on
    IntOp $0 $0 + 1
    ReadRegStr $R0 HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\$1" "DisplayName"
    ReadRegStr $R1 HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\$1" "Publisher"
    StrCmp "$R0$R1" "${PRODUCTNAME}${MANUFACTURER}" 0 wix_loop
    ReadRegStr $R0 HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\$1" "UninstallString"
    ${StrCase} $R1 $R0 "L"
    ${StrLoc} $R0 $R1 "msiexec" ">"
    StrCmp $R0 0 0 wix_loop_done
    StrCpy $R6 "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\$1"
  wix_loop_done:
FunctionEnd

Function PageReinstall
  ; Uninstall previous WiX installation if exists.
  Call FindWixInstall
  ${If} $R6 != ""
    StrCpy $WixMode 1
    Goto compare_version
  ${EndIf}

  ; Check if there is an existing installation, if not, abort the reinstall page
  ReadRegStr $R0 SHCTX "${UNINSTKEY}" ""
  ReadRegStr $R1 SHCTX "${UNINSTKEY}" "UninstallString"
  ${IfThen} "$R0$R1" == "" ${|} Abort ${|}

  ; Compare this installar version with the existing installation
  ; and modify the messages presented to the user accordingly
  compare_version:
  StrCpy $R4 "$(older)"
  ${If} $WixMode = 1
    ReadRegStr $R0 HKLM "$R6" "DisplayVersion"
  ${Else}
    ReadRegStr $R0 SHCTX "${UNINSTKEY}" "DisplayVersion"
  ${EndIf}
  ${IfThen} $R0 == "" ${|} StrCpy $R4 "$(unknown)" ${|}

  nsis_tauri_utils::SemverCompare "${VERSION}" $R0
  Pop $R0

  ; Read the install choice header through $R5 so a custom language file that only
  ; defines the misspelled `choowHowToInstall` still shows its own text
  ; TODO: remove the `choowHowToInstall` fallback in a future major version
  StrCpy $R5 "$(chooseHowToInstall)"
  ${IfThen} $R5 == "" ${|} StrCpy $R5 "$(choowHowToInstall)" ${|}

  ; Reinstalling the same version
  ${If} $R0 = 0
    StrCpy $R1 "$(alreadyInstalledLong)"
    StrCpy $R2 "$(addOrReinstall)"
    StrCpy $R3 "$(uninstallApp)"
    !insertmacro MUI_HEADER_TEXT "$(alreadyInstalled)" "$(chooseMaintenanceOption)"
  ; Upgrading
  ${ElseIf} $R0 = 1
    StrCpy $R1 "$(olderOrUnknownVersionInstalled)"
    StrCpy $R2 "$(uninstallBeforeInstalling)"
    StrCpy $R3 "$(auroraUpdate)"
    !insertmacro MUI_HEADER_TEXT "$(alreadyInstalled)" "$R5"
  ; Downgrading
  ${ElseIf} $R0 = -1
    StrCpy $R1 "$(newerVersionInstalled)"
    StrCpy $R2 "$(uninstallBeforeInstalling)"
    !if "${ALLOWDOWNGRADES}" == "true"
      StrCpy $R3 "$(dontUninstall)"
    !else
      StrCpy $R3 "$(dontUninstallDowngrade)"
    !endif
    !insertmacro MUI_HEADER_TEXT "$(alreadyInstalled)" "$R5"
  ${Else}
    Abort
  ${EndIf}

  ; Skip showing the page if passive
  ;
  ; Note that we don't call this earlier at the beginning
  ; of this function because we need to populate some variables
  ; related to current installed version if detected and whether
  ; we are downgrading or not.
  ${If} $PassiveMode = 1
    Call PageLeaveReinstall
  ${Else}
    nsDialogs::Create 1018
    Pop $R4
    ${IfThen} $(^RTL) = 1 ${|} nsDialogs::SetRTL $(^RTL) ${|}

    ${NSD_CreateLabel} 0 0 100% 24u $R1
    Pop $R1

    ; AURORA: nell'aggiornamento «Aggiorna» ($R3) sta sopra e «Disinstalla prima» sotto
    StrCpy $AuroraTop 50u
    StrCpy $AuroraLow 70u
    ${If} $R0 = 1
      StrCpy $AuroraTop 70u
      StrCpy $AuroraLow 50u
    ${EndIf}
    ${NSD_CreateRadioButton} 30u $AuroraTop -30u 10u $R2
    Pop $R2
    ${NSD_OnClick} $R2 PageReinstallUpdateSelection

    ${NSD_CreateRadioButton} 30u $AuroraLow -30u 10u $R3
    Pop $R3
    ; Disable this radio button if downgrading and downgrades are disabled
    !if "${ALLOWDOWNGRADES}" == "false"
      ${IfThen} $R0 = -1 ${|} EnableWindow $R3 0 ${|}
    !endif
    ${NSD_OnClick} $R3 PageReinstallUpdateSelection

    ; AURORA: con una versione piu' vecchia installata la scelta di base e' l'aggiornamento («Non disinstallare»);
    ; disinstallare prima resta una scelta da fare a mano (richiesta del proprietario, 2026-11-08)
    ${If} $ReinstallPageCheck = 0
    ${AndIf} $R0 = 1
      StrCpy $ReinstallPageCheck 2
    ${EndIf}
    ; Check the first radio button if this the first time
    ; we enter this page or if the second button wasn't
    ; selected the last time we were on this page
    ${If} $ReinstallPageCheck <> 2
      SendMessage $R2 ${BM_SETCHECK} ${BST_CHECKED} 0
    ${Else}
      SendMessage $R3 ${BM_SETCHECK} ${BST_CHECKED} 0
    ${EndIf}

    ${If} $ReinstallPageCheck = 2
      ${NSD_SetFocus} $R3
    ${Else}
      ${NSD_SetFocus} $R2
    ${EndIf}
    nsDialogs::Show
  ${EndIf}
FunctionEnd
Function PageReinstallUpdateSelection
  ${NSD_GetState} $R2 $R1
  ${If} $R1 == ${BST_CHECKED}
    StrCpy $ReinstallPageCheck 1
  ${Else}
    StrCpy $ReinstallPageCheck 2
  ${EndIf}
FunctionEnd
Function PageLeaveReinstall
  ${NSD_GetState} $R2 $R1

  ; If migrating from Wix, always uninstall
  ${If} $WixMode = 1
    Goto reinst_uninstall
  ${EndIf}

  ; In update mode, always proceeds without uninstalling
  ${If} $UpdateMode = 1
    Goto reinst_done
  ${EndIf}

  ; $R0 holds whether same(0)/upgrading(1)/downgrading(-1) version
  ; $R1 holds the radio buttons state:
  ;   1 => first choice was selected
  ;   0 => second choice was selected
  ${If} $R0 = 0 ; Same version, proceed
    ${If} $R1 = 1              ; User chose to add/reinstall
      Goto reinst_done
    ${Else}                    ; User chose to uninstall
      Goto reinst_uninstall
    ${EndIf}
  ${ElseIf} $R0 = 1 ; Upgrading
    ${If} $R1 = 1              ; User chose to uninstall
      Goto reinst_uninstall
    ${Else}
      Goto reinst_done         ; User chose NOT to uninstall
    ${EndIf}
  ${ElseIf} $R0 = -1 ; Downgrading
    ${If} $R1 = 1              ; User chose to uninstall
      Goto reinst_uninstall
    ${Else}
      Goto reinst_done         ; User chose NOT to uninstall
    ${EndIf}
  ${EndIf}

  reinst_uninstall:
    HideWindow
    ClearErrors

    ${If} $WixMode = 1
      ReadRegStr $R1 HKLM "$R6" "UninstallString"
      ExecWait '$R1' $0
    ${Else}
      ReadRegStr $4 SHCTX "${MANUPRODUCTKEY}" ""
      ReadRegStr $R1 SHCTX "${UNINSTKEY}" "UninstallString"
      ${IfThen} $UpdateMode = 1 ${|} StrCpy $R1 "$R1 /UPDATE" ${|} ; append /UPDATE
      ${IfThen} $PassiveMode = 1 ${|} StrCpy $R1 "$R1 /P" ${|} ; append /P
      StrCpy $R1 "$R1 _?=$4" ; append uninstall directory
      ExecWait '$R1' $0
    ${EndIf}

    BringToFront

    ${IfThen} ${Errors} ${|} StrCpy $0 2 ${|} ; ExecWait failed, set fake exit code

    ${If} $0 <> 0
    ${OrIf} ${FileExists} "$INSTDIR\${MAINBINARYNAME}.exe"
      ; User cancelled wix uninstaller? return to select un/reinstall page
      ${If} $WixMode = 1
      ${AndIf} $0 = 1602
        Abort
      ${EndIf}

      ; User cancelled NSIS uninstaller? return to select un/reinstall page
      ${If} $0 = 1
        Abort
      ${EndIf}

      ; Other errors? show generic error message and return to select un/reinstall page
      MessageBox MB_ICONEXCLAMATION "$(unableToUninstall)"
      Abort
    ${EndIf}

    ; The NSIS uninstaller was run in place (`_?=`), so it cannot delete itself
    ; and the install directory, clean them up here once the uninstall succeeded
    ; https://nsis.sourceforge.io/Docs/Chapter3.html#:~:text=path%20contains%20spaces.-,3.2.3%20Examples,-installer.exe%20/NCRC
    ${If} $WixMode = 0
    ${AndIf} $4 != ""
      Delete "$4\uninstall.exe"
      RMDir "$4"
    ${EndIf}
  reinst_done:
FunctionEnd

; 5. Choose install directory page
!define MUI_PAGE_CUSTOMFUNCTION_PRE SkipIfPassive
!insertmacro MUI_PAGE_DIRECTORY

; 6. Start menu shortcut page
Var AppStartMenuFolder
!if "${STARTMENUFOLDER}" != ""
  !define MUI_PAGE_CUSTOMFUNCTION_PRE SkipIfPassive
  !define MUI_STARTMENUPAGE_DEFAULTFOLDER "${STARTMENUFOLDER}"
!else
  !define MUI_PAGE_CUSTOMFUNCTION_PRE Skip
!endif
!insertmacro MUI_PAGE_STARTMENU Application $AppStartMenuFolder

; 7. Installation page
; AURORA: il titolo della pagina quando l'installazione non riesce (il motivo e cosa fare li dice AuroraSpiega)
!define MUI_INSTFILESPAGE_ABORTHEADER_TEXT "$(auroraFailTitle)"
!insertmacro MUI_PAGE_INSTFILES

; 8. Finish page
;
; Don't auto jump to finish page after installation page,
; because the installation page has useful info that can be used debug any issues with the installer.
!define MUI_FINISHPAGE_NOAUTOCLOSE
; Use show readme button in the finish page as a button create a desktop shortcut
!define MUI_FINISHPAGE_SHOWREADME
!define MUI_FINISHPAGE_SHOWREADME_TEXT "$(createDesktop)"
!define MUI_FINISHPAGE_SHOWREADME_FUNCTION CreateOrUpdateDesktopShortcut
; Show run app after installation.
!define MUI_FINISHPAGE_RUN
!define MUI_FINISHPAGE_RUN_FUNCTION RunMainBinary
!define MUI_PAGE_CUSTOMFUNCTION_PRE SkipIfPassive
!insertmacro MUI_PAGE_FINISH

Function RunMainBinary
  nsis_tauri_utils::RunAsUser "$INSTDIR\${MAINBINARYNAME}.exe" ""
FunctionEnd

; Uninstaller Pages
; 1. Confirm uninstall page
Var DeleteAppDataCheckbox
Var DeleteAppDataCheckboxState
!define /ifndef WS_EX_LAYOUTRTL         0x00400000
!define MUI_PAGE_CUSTOMFUNCTION_SHOW un.ConfirmShow
Function un.ConfirmShow ; Add add a `Delete app data` check box
  ; AURORA: niente casella «Elimina i dati dell'applicazione»: la disinstallazione non cancella mai configurazione e dati
  Return
  ; $1 inner dialog HWND
  ; $2 window DPI
  ; $3 style
  ; $4 x
  ; $5 y
  ; $6 width
  ; $7 height
  FindWindow $1 "#32770" "" $HWNDPARENT ; Find inner dialog
  System::Call "user32::GetDpiForWindow(p r1) i .r2"
  ${If} $(^RTL) = 1
    StrCpy $3 "${__NSD_CheckBox_EXSTYLE} | ${WS_EX_LAYOUTRTL}"
    IntOp $4 50 * $2
  ${Else}
    StrCpy $3 "${__NSD_CheckBox_EXSTYLE}"
    IntOp $4 0 * $2
  ${EndIf}
  IntOp $5 100 * $2
  IntOp $6 400 * $2
  IntOp $7 25 * $2
  IntOp $4 $4 / 96
  IntOp $5 $5 / 96
  IntOp $6 $6 / 96
  IntOp $7 $7 / 96
  System::Call 'user32::CreateWindowEx(i r3, w "${__NSD_CheckBox_CLASS}", w "$(deleteAppData)", i ${__NSD_CheckBox_STYLE}, i r4, i r5, i r6, i r7, p r1, i0, i0, i0) i .s'
  Pop $DeleteAppDataCheckbox
  SendMessage $HWNDPARENT ${WM_GETFONT} 0 0 $1
  SendMessage $DeleteAppDataCheckbox ${WM_SETFONT} $1 1
FunctionEnd
!define MUI_PAGE_CUSTOMFUNCTION_LEAVE un.ConfirmLeave
Function un.ConfirmLeave
  SendMessage $DeleteAppDataCheckbox ${BM_GETCHECK} 0 0 $DeleteAppDataCheckboxState
FunctionEnd
!define MUI_PAGE_CUSTOMFUNCTION_PRE un.SkipIfPassive
!insertmacro MUI_UNPAGE_CONFIRM

; 2. Uninstalling Page
!insertmacro MUI_UNPAGE_INSTFILES

;Languages
; PageReinstall reads the misspelled `choowHowToInstall` as a fallback, and makensis warns
; about a language string that is read but never set, so default it to an empty string for
; every language. This runs before the language files are included, so a custom language
; file that only defines the old spelling overrides the default and the fallback shows its
; text. `0` as the language id means the language inserted right above.
; TODO: remove the `choowHowToInstall` fallback in a future major version
{{#each languages}}
!insertmacro MUI_LANGUAGE "{{this}}"
LangString choowHowToInstall 0 ""
{{/each}}
!insertmacro MUI_RESERVEFILE_LANGDLL
; AURORA: con una versione piu' vecchia installata la scelta si chiama «Aggiorna» e sta sopra
LangString auroraUpdate ${LANG_ITALIAN} "Aggiorna (consigliato)"
LangString auroraUpdate ${LANG_ENGLISH} "Update (recommended)"
LangString auroraUpdate ${LANG_GERMAN} "Aktualisieren (empfohlen)"
LangString auroraUpdate ${LANG_FRENCH} "Mettre à jour (recommandé)"
LangString auroraUpdate ${LANG_SPANISH} "Actualizar (recomendado)"
LangString auroraUpdate ${LANG_PORTUGUESE} "Atualizar (recomendado)"
LangString auroraUpdate ${LANG_DUTCH} "Bijwerken (aanbevolen)"
; AURORA: registro dell'installazione e rapporto anonimo quando l'installazione non riesce (hooks.nsh)
LangString auroraLogSaved ${LANG_ITALIAN} "Registro dell'installazione:"
LangString auroraLogSaved ${LANG_ENGLISH} "Installation log:"
LangString auroraLogSaved ${LANG_GERMAN} "Installationsprotokoll:"
LangString auroraLogSaved ${LANG_FRENCH} "Journal de l'installation :"
LangString auroraLogSaved ${LANG_SPANISH} "Registro de la instalación:"
LangString auroraLogSaved ${LANG_PORTUGUESE} "Registo da instalação:"
LangString auroraLogSaved ${LANG_DUTCH} "Installatielogboek:"
LangString auroraReportSending ${LANG_ITALIAN} "Invio del rapporto anonimo sull'installazione non riuscita a chi sviluppa Aurora..."
LangString auroraReportSending ${LANG_ENGLISH} "Sending the anonymous report about the failed installation to the Aurora developer..."
LangString auroraReportSending ${LANG_GERMAN} "Anonymer Bericht über die fehlgeschlagene Installation wird an den Entwickler von Aurora gesendet..."
LangString auroraReportSending ${LANG_FRENCH} "Envoi du rapport anonyme sur l'installation échouée au développeur d'Aurora..."
LangString auroraReportSending ${LANG_SPANISH} "Enviando el informe anónimo de la instalación fallida al desarrollador de Aurora..."
LangString auroraReportSending ${LANG_PORTUGUESE} "A enviar o relatório anónimo da instalação falhada ao programador do Aurora..."
LangString auroraReportSending ${LANG_DUTCH} "Anoniem rapport over de mislukte installatie wordt naar de ontwikkelaar van Aurora gestuurd..."
LangString auroraReportSent ${LANG_ITALIAN} "Rapporto anonimo mandato (senza nome utente, nome del PC né percorsi personali). Si spengono in Aurora: Impostazioni > Accesso e assistenza > Assistenza."
LangString auroraReportSent ${LANG_ENGLISH} "Anonymous report sent (without user name, PC name or personal paths). Turn reports off in Aurora: Settings > Access and support > Support."
LangString auroraReportSent ${LANG_GERMAN} "Anonymer Bericht gesendet (ohne Benutzernamen, PC-Namen oder persönliche Pfade). Abschalten in Aurora: Einstellungen > Zugang und Hilfe > Hilfe."
LangString auroraReportSent ${LANG_FRENCH} "Rapport anonyme envoyé (sans nom d'utilisateur, nom du PC ni chemins personnels). Désactivation dans Aurora : Réglages > Accès et assistance > Assistance."
LangString auroraReportSent ${LANG_SPANISH} "Informe anónimo enviado (sin nombre de usuario, nombre del PC ni rutas personales). Se desactivan en Aurora: Ajustes > Acceso y asistencia > Asistencia."
LangString auroraReportSent ${LANG_PORTUGUESE} "Relatório anónimo enviado (sem nome de utilizador, nome do PC nem caminhos pessoais). Desative-os no Aurora: Definições > Acesso e assistência > Assistência."
LangString auroraReportSent ${LANG_DUTCH} "Anoniem rapport verstuurd (zonder gebruikersnaam, pc-naam of persoonlijke paden). Uitzetten in Aurora: Instellingen > Toegang en ondersteuning > Ondersteuning."
; AURORA: la schermata finale di un'installazione non riuscita (AuroraSpiega in hooks.nsh)
LangString auroraFailTitle ${LANG_ITALIAN} "Aurora non è stata installata."
LangString auroraFailTitle ${LANG_ENGLISH} "Aurora was not installed."
LangString auroraFailTitle ${LANG_GERMAN} "Aurora wurde nicht installiert."
LangString auroraFailTitle ${LANG_FRENCH} "Aurora n'a pas été installée."
LangString auroraFailTitle ${LANG_SPANISH} "Aurora no se ha instalado."
LangString auroraFailTitle ${LANG_PORTUGUESE} "O Aurora não foi instalado."
LangString auroraFailTitle ${LANG_DUTCH} "Aurora is niet geïnstalleerd."
LangString auroraFailWebview2 ${LANG_ITALIAN} "Motivo: non si è potuto installare Microsoft Edge WebView2, il componente di Windows con cui Aurora disegna la sua finestra.$\r$\nCosa fare: controlla di essere collegato a Internet e riprova. Se non basta, installa WebView2 dalla pagina di Microsoft (cerca «WebView2 Runtime») e poi riprova."
LangString auroraFailWebview2 ${LANG_ENGLISH} "Reason: Microsoft Edge WebView2, the Windows component Aurora uses to draw its window, could not be installed.$\r$\nWhat to do: check that you are connected to the Internet and try again. If that is not enough, install WebView2 from Microsoft's page (search for “WebView2 Runtime”) and try again."
LangString auroraFailWebview2 ${LANG_GERMAN} "Grund: Microsoft Edge WebView2, die Windows-Komponente, mit der Aurora sein Fenster zeichnet, konnte nicht installiert werden.$\r$\nWas tun: Prüfe die Internetverbindung und versuche es erneut. Reicht das nicht, installiere WebView2 von der Seite von Microsoft (Suche nach „WebView2 Runtime“) und versuche es dann erneut."
LangString auroraFailWebview2 ${LANG_FRENCH} "Motif : impossible d'installer Microsoft Edge WebView2, le composant de Windows avec lequel Aurora dessine sa fenêtre.$\r$\nQue faire : vérifie que tu es connecté à Internet et réessaie. Si cela ne suffit pas, installe WebView2 depuis la page de Microsoft (cherche « WebView2 Runtime ») puis réessaie."
LangString auroraFailWebview2 ${LANG_SPANISH} "Motivo: no se ha podido instalar Microsoft Edge WebView2, el componente de Windows con el que Aurora dibuja su ventana.$\r$\nQué hacer: comprueba que estás conectado a Internet y vuelve a intentarlo. Si no basta, instala WebView2 desde la página de Microsoft (busca «WebView2 Runtime») y vuelve a intentarlo."
LangString auroraFailWebview2 ${LANG_PORTUGUESE} "Motivo: não foi possível instalar o Microsoft Edge WebView2, o componente do Windows com que o Aurora desenha a sua janela.$\r$\nO que fazer: verifica que estás ligado à Internet e tenta de novo. Se não chegar, instala o WebView2 a partir da página da Microsoft (procura «WebView2 Runtime») e tenta de novo."
LangString auroraFailWebview2 ${LANG_DUTCH} "Reden: Microsoft Edge WebView2, het Windows-onderdeel waarmee Aurora zijn venster tekent, kon niet worden geïnstalleerd.$\r$\nWat te doen: controleer of je met internet verbonden bent en probeer het opnieuw. Is dat niet genoeg, installeer WebView2 via de pagina van Microsoft (zoek op „WebView2 Runtime”) en probeer het daarna opnieuw."
LangString auroraFailVersion ${LANG_ITALIAN} "Motivo: sul PC c'è già una versione di Aurora più nuova di questa.$\r$\nCosa fare: non serve fare niente. Per tornare a questa versione disinstalla prima Aurora (i tuoi dati restano)."
LangString auroraFailVersion ${LANG_ENGLISH} "Reason: a newer version of Aurora is already installed on this PC.$\r$\nWhat to do: nothing is needed. To go back to this version, uninstall Aurora first (your data stays)."
LangString auroraFailVersion ${LANG_GERMAN} "Grund: Auf diesem PC ist bereits eine neuere Version von Aurora installiert.$\r$\nWas tun: Nichts nötig. Um zu dieser Version zurückzukehren, zuerst Aurora deinstallieren (deine Daten bleiben erhalten)."
LangString auroraFailVersion ${LANG_FRENCH} "Motif : une version plus récente d'Aurora est déjà installée sur ce PC.$\r$\nQue faire : rien. Pour revenir à cette version, désinstalle d'abord Aurora (tes données restent)."
LangString auroraFailVersion ${LANG_SPANISH} "Motivo: en el PC ya hay una versión de Aurora más nueva que esta.$\r$\nQué hacer: no hace falta nada. Para volver a esta versión desinstala antes Aurora (tus datos se conservan)."
LangString auroraFailVersion ${LANG_PORTUGUESE} "Motivo: no PC já existe uma versão do Aurora mais recente do que esta.$\r$\nO que fazer: não é preciso nada. Para voltar a esta versão desinstala primeiro o Aurora (os teus dados ficam)."
LangString auroraFailVersion ${LANG_DUTCH} "Reden: op deze pc staat al een nieuwere versie van Aurora.$\r$\nWat te doen: niets nodig. Wil je terug naar deze versie, verwijder dan eerst Aurora (je gegevens blijven bewaard)."
LangString auroraFailFiles ${LANG_ITALIAN} "Motivo: i file di Aurora erano bloccati da un altro programma (Aurora ancora aperta, l'antivirus o un servizio di Windows).$\r$\nCosa fare: chiudi Aurora, anche l'icona vicino all'orologio, aspetta qualche secondo e riprova. Se si ripete, riavvia il PC e riprova."
LangString auroraFailFiles ${LANG_ENGLISH} "Reason: Aurora's files were locked by another program (Aurora still open, the antivirus or a Windows service).$\r$\nWhat to do: close Aurora, including the icon near the clock, wait a few seconds and try again. If it happens again, restart the PC and try again."
LangString auroraFailFiles ${LANG_GERMAN} "Grund: Die Dateien von Aurora waren durch ein anderes Programm gesperrt (Aurora noch offen, der Virenschutz oder ein Windows-Dienst).$\r$\nWas tun: Aurora schließen, auch das Symbol neben der Uhr, ein paar Sekunden warten und erneut versuchen. Wiederholt es sich, den PC neu starten und erneut versuchen."
LangString auroraFailFiles ${LANG_FRENCH} "Motif : les fichiers d'Aurora étaient bloqués par un autre programme (Aurora encore ouverte, l'antivirus ou un service de Windows).$\r$\nQue faire : ferme Aurora, y compris l'icône près de l'horloge, attends quelques secondes et réessaie. Si cela se répète, redémarre le PC et réessaie."
LangString auroraFailFiles ${LANG_SPANISH} "Motivo: los archivos de Aurora estaban bloqueados por otro programa (Aurora aún abierta, el antivirus o un servicio de Windows).$\r$\nQué hacer: cierra Aurora, también el icono junto al reloj, espera unos segundos y vuelve a intentarlo. Si se repite, reinicia el PC y vuelve a intentarlo."
LangString auroraFailFiles ${LANG_PORTUGUESE} "Motivo: os ficheiros do Aurora estavam bloqueados por outro programa (o Aurora ainda aberto, o antivírus ou um serviço do Windows).$\r$\nO que fazer: fecha o Aurora, também o ícone junto ao relógio, espera alguns segundos e tenta de novo. Se se repetir, reinicia o PC e tenta de novo."
LangString auroraFailFiles ${LANG_DUTCH} "Reden: de bestanden van Aurora waren vergrendeld door een ander programma (Aurora nog open, de virusscanner of een Windows-service).$\r$\nWat te doen: sluit Aurora, ook het pictogram bij de klok, wacht een paar seconden en probeer het opnieuw. Gebeurt het weer, start de pc opnieuw op en probeer het opnieuw."
LangString auroraFailSpace ${LANG_ITALIAN} "Motivo: sul disco non c'è abbastanza spazio libero.$\r$\nCosa fare: libera almeno 500 MB (per esempio svuotando il Cestino) e riprova."
LangString auroraFailSpace ${LANG_ENGLISH} "Reason: there is not enough free space on the disk.$\r$\nWhat to do: free at least 500 MB (for example by emptying the Recycle Bin) and try again."
LangString auroraFailSpace ${LANG_GERMAN} "Grund: Auf der Festplatte ist nicht genug freier Platz.$\r$\nWas tun: Mindestens 500 MB freigeben (zum Beispiel den Papierkorb leeren) und erneut versuchen."
LangString auroraFailSpace ${LANG_FRENCH} "Motif : il n'y a pas assez d'espace libre sur le disque.$\r$\nQue faire : libère au moins 500 Mo (par exemple en vidant la Corbeille) et réessaie."
LangString auroraFailSpace ${LANG_SPANISH} "Motivo: no hay suficiente espacio libre en el disco.$\r$\nQué hacer: libera al menos 500 MB (por ejemplo vaciando la Papelera) y vuelve a intentarlo."
LangString auroraFailSpace ${LANG_PORTUGUESE} "Motivo: não há espaço livre suficiente no disco.$\r$\nO que fazer: liberta pelo menos 500 MB (por exemplo esvaziando a Reciclagem) e tenta de novo."
LangString auroraFailSpace ${LANG_DUTCH} "Reden: er is niet genoeg vrije ruimte op de schijf.$\r$\nWat te doen: maak minstens 500 MB vrij (bijvoorbeeld door de Prullenbak te legen) en probeer het opnieuw."
LangString auroraFailAntivirus ${LANG_ITALIAN} "Motivo: il programma di Aurora è stato tolto subito dopo la copia, quasi sempre dall'antivirus.$\r$\nCosa fare: aggiungi la cartella di Aurora alle eccezioni dell'antivirus e riprova. Aurora è firmato e il suo codice è pubblico su GitHub."
LangString auroraFailAntivirus ${LANG_ENGLISH} "Reason: Aurora's program was removed right after being copied, almost always by the antivirus.$\r$\nWhat to do: add Aurora's folder to the antivirus exceptions and try again. Aurora is signed and its code is public on GitHub."
LangString auroraFailAntivirus ${LANG_GERMAN} "Grund: Das Programm von Aurora wurde gleich nach dem Kopieren entfernt, fast immer vom Virenschutz.$\r$\nWas tun: Den Ordner von Aurora zu den Ausnahmen des Virenschutzes hinzufügen und erneut versuchen. Aurora ist signiert und sein Code ist auf GitHub öffentlich."
LangString auroraFailAntivirus ${LANG_FRENCH} "Motif : le programme d'Aurora a été retiré juste après la copie, presque toujours par l'antivirus.$\r$\nQue faire : ajoute le dossier d'Aurora aux exceptions de l'antivirus et réessaie. Aurora est signée et son code est public sur GitHub."
LangString auroraFailAntivirus ${LANG_SPANISH} "Motivo: el programa de Aurora se eliminó justo después de copiarlo, casi siempre por el antivirus.$\r$\nQué hacer: añade la carpeta de Aurora a las excepciones del antivirus y vuelve a intentarlo. Aurora está firmada y su código es público en GitHub."
LangString auroraFailAntivirus ${LANG_PORTUGUESE} "Motivo: o programa do Aurora foi removido logo depois de ser copiado, quase sempre pelo antivírus.$\r$\nO que fazer: adiciona a pasta do Aurora às exceções do antivírus e tenta de novo. O Aurora é assinado e o seu código é público no GitHub."
LangString auroraFailAntivirus ${LANG_DUTCH} "Reden: het programma van Aurora is direct na het kopiëren verwijderd, bijna altijd door de virusscanner.$\r$\nWat te doen: voeg de map van Aurora toe aan de uitzonderingen van de virusscanner en probeer het opnieuw. Aurora is ondertekend en de code staat openbaar op GitHub."
LangString auroraFailOther ${LANG_ITALIAN} "Motivo: l'installazione si è fermata durante: $AuroraPasso.$\r$\nCosa fare: riprova. Se si ripete, apri una segnalazione su GitHub e allega il registro dell'installazione."
LangString auroraFailOther ${LANG_ENGLISH} "Reason: the installation stopped during: $AuroraPasso.$\r$\nWhat to do: try again. If it happens again, open an issue on GitHub and attach the installation log."
LangString auroraFailOther ${LANG_GERMAN} "Grund: Die Installation wurde angehalten bei: $AuroraPasso.$\r$\nWas tun: Erneut versuchen. Wiederholt es sich, auf GitHub eine Meldung öffnen und das Installationsprotokoll anhängen."
LangString auroraFailOther ${LANG_FRENCH} "Motif : l'installation s'est arrêtée pendant : $AuroraPasso.$\r$\nQue faire : réessaie. Si cela se répète, ouvre un signalement sur GitHub et joins le journal de l'installation."
LangString auroraFailOther ${LANG_SPANISH} "Motivo: la instalación se detuvo durante: $AuroraPasso.$\r$\nQué hacer: vuelve a intentarlo. Si se repite, abre un aviso en GitHub y adjunta el registro de la instalación."
LangString auroraFailOther ${LANG_PORTUGUESE} "Motivo: a instalação parou durante: $AuroraPasso.$\r$\nO que fazer: tenta de novo. Se se repetir, abre um relato no GitHub e anexa o registo da instalação."
LangString auroraFailOther ${LANG_DUTCH} "Reden: de installatie stopte tijdens: $AuroraPasso.$\r$\nWat te doen: probeer het opnieuw. Gebeurt het weer, open een melding op GitHub en voeg het installatielogboek toe."
LangString auroraFailReportSent ${LANG_ITALIAN} "È partito un rapporto anonimo (senza nome utente, nome del PC né percorsi personali): serve a capire e correggere il problema."
LangString auroraFailReportSent ${LANG_ENGLISH} "An anonymous report was sent (without user name, PC name or personal paths): it helps to understand and fix the problem."
LangString auroraFailReportSent ${LANG_GERMAN} "Ein anonymer Bericht wurde gesendet (ohne Benutzernamen, PC-Namen oder persönliche Pfade): er hilft, das Problem zu verstehen und zu beheben."
LangString auroraFailReportSent ${LANG_FRENCH} "Un rapport anonyme a été envoyé (sans nom d'utilisateur, nom du PC ni chemins personnels) : il aide à comprendre et corriger le problème."
LangString auroraFailReportSent ${LANG_SPANISH} "Se ha enviado un informe anónimo (sin nombre de usuario, nombre del PC ni rutas personales): sirve para entender y corregir el problema."
LangString auroraFailReportSent ${LANG_PORTUGUESE} "Foi enviado um relatório anónimo (sem nome de utilizador, nome do PC nem caminhos pessoais): ajuda a perceber e corrigir o problema."
LangString auroraFailReportSent ${LANG_DUTCH} "Er is een anoniem rapport verstuurd (zonder gebruikersnaam, pc-naam of persoonlijke paden): het helpt het probleem te begrijpen en op te lossen."
LangString auroraFailReportOff ${LANG_ITALIAN} "Nessun rapporto anonimo: i rapporti sono spenti in Aurora."
LangString auroraFailReportOff ${LANG_ENGLISH} "No anonymous report: reports are turned off in Aurora."
LangString auroraFailReportOff ${LANG_GERMAN} "Kein anonymer Bericht: Berichte sind in Aurora abgeschaltet."
LangString auroraFailReportOff ${LANG_FRENCH} "Aucun rapport anonyme : les rapports sont désactivés dans Aurora."
LangString auroraFailReportOff ${LANG_SPANISH} "Ningún informe anónimo: los informes están desactivados en Aurora."
LangString auroraFailReportOff ${LANG_PORTUGUESE} "Nenhum relatório anónimo: os relatórios estão desligados no Aurora."
LangString auroraFailReportOff ${LANG_DUTCH} "Geen anoniem rapport: rapporten staan uit in Aurora."
LangString auroraFailReportNot ${LANG_ITALIAN} "Il rapporto anonimo non è partito (niente Internet?): se il problema si ripete, apri una segnalazione su GitHub con il registro."
LangString auroraFailReportNot ${LANG_ENGLISH} "The anonymous report could not be sent (no Internet?): if the problem happens again, open an issue on GitHub with the log."
LangString auroraFailReportNot ${LANG_GERMAN} "Der anonyme Bericht konnte nicht gesendet werden (kein Internet?): wiederholt sich das Problem, auf GitHub eine Meldung mit dem Protokoll öffnen."
LangString auroraFailReportNot ${LANG_FRENCH} "Le rapport anonyme n'a pas pu partir (pas d'Internet ?) : si le problème se répète, ouvre un signalement sur GitHub avec le journal."
LangString auroraFailReportNot ${LANG_SPANISH} "El informe anónimo no se ha podido enviar (¿sin Internet?): si el problema se repite, abre un aviso en GitHub con el registro."
LangString auroraFailReportNot ${LANG_PORTUGUESE} "O relatório anónimo não foi enviado (sem Internet?): se o problema se repetir, abre um relato no GitHub com o registo."
LangString auroraFailReportNot ${LANG_DUTCH} "Het anonieme rapport kon niet worden verstuurd (geen internet?): gebeurt het weer, open dan een melding op GitHub met het logboek."
LangString auroraFailAsk ${LANG_ITALIAN} "Sì = riprova adesso    No = apri il registro e le segnalazioni    Annulla = chiudi"
LangString auroraFailAsk ${LANG_ENGLISH} "Yes = try again now    No = open the log and the issues page    Cancel = close"
LangString auroraFailAsk ${LANG_GERMAN} "Ja = jetzt erneut versuchen    Nein = Protokoll und Meldungen öffnen    Abbrechen = schließen"
LangString auroraFailAsk ${LANG_FRENCH} "Oui = réessayer maintenant    Non = ouvrir le journal et les signalements    Annuler = fermer"
LangString auroraFailAsk ${LANG_SPANISH} "Sí = volver a intentarlo    No = abrir el registro y los avisos    Cancelar = cerrar"
LangString auroraFailAsk ${LANG_PORTUGUESE} "Sim = tentar de novo agora    Não = abrir o registo e os relatos    Cancelar = fechar"
LangString auroraFailAsk ${LANG_DUTCH} "Ja = nu opnieuw proberen    Nee = logboek en meldingen openen    Annuleren = sluiten"
!insertmacro AuroraFunzioniRegistro
{{#each language_files}}
  !include "{{this}}"
{{/each}}

Function .onInit
  ${GetOptions} $CMDLINE "/P" $PassiveMode
  ${IfNot} ${Errors}
    StrCpy $PassiveMode 1
  ${EndIf}

  ${GetOptions} $CMDLINE "/NS" $NoShortcutMode
  ${IfNot} ${Errors}
    StrCpy $NoShortcutMode 1
  ${EndIf}

  ${GetOptions} $CMDLINE "/UPDATE" $UpdateMode
  ${IfNot} ${Errors}
    StrCpy $UpdateMode 1
  ${EndIf}

  !if "${DISPLAYLANGUAGESELECTOR}" == "true"
    !insertmacro MUI_LANGDLL_DISPLAY
  !endif

  !insertmacro SetContext

  ; AURORA: la versione gia' installata, per il registro (la sezione Install la sovrascrive)
  ReadRegStr $AuroraVersionePrima SHCTX "${UNINSTKEY}" "DisplayVersion"

  ${If} $INSTDIR == "${PLACEHOLDER_INSTALL_DIR}"
    ; Set default install location
    !if "${INSTALLMODE}" == "perMachine"
      ${If} ${RunningX64}
        !if "${ARCH}" == "x64"
          StrCpy $INSTDIR "$PROGRAMFILES64\${PRODUCTNAME}"
        !else if "${ARCH}" == "arm64"
          StrCpy $INSTDIR "$PROGRAMFILES64\${PRODUCTNAME}"
        !else
          StrCpy $INSTDIR "$PROGRAMFILES\${PRODUCTNAME}"
        !endif
      ${Else}
        StrCpy $INSTDIR "$PROGRAMFILES\${PRODUCTNAME}"
      ${EndIf}
    !else if "${INSTALLMODE}" == "currentUser"
      StrCpy $INSTDIR "$LOCALAPPDATA\${PRODUCTNAME}"
    !endif

    Call RestorePreviousInstallLocation
  ${EndIf}


  !if "${INSTALLMODE}" == "both"
    !insertmacro MULTIUSER_INIT
  !endif

  ; Abort downgrades if they are disabled, in the modes that skip the reinstall page choice:
  ; silent, passive and update
  !if "${ALLOWDOWNGRADES}" == "false"
  ${If} ${Silent}
  ${OrIf} $PassiveMode = 1
  ${OrIf} $UpdateMode = 1
    ; Compare with the same installation the reinstall page would: a WiX one first
    Call FindWixInstall
    ${If} $R6 != ""
      ReadRegStr $R0 HKLM "$R6" "DisplayVersion"
    ${Else}
      ReadRegStr $R0 SHCTX "${UNINSTKEY}" "DisplayVersion"
    ${EndIf}
    nsis_tauri_utils::SemverCompare "${VERSION}" $R0
    Pop $R0
    ; If downgrading
    ${If} $R0 = -1
      ${If} ${Silent}
        System::Call 'kernel32::AttachConsole(i -1)i.r0'
        ${If} $0 <> 0
          System::Call 'kernel32::GetStdHandle(i -11)i.r0'
          System::call 'kernel32::SetConsoleTextAttribute(i r0, i 0x0004)' ; set red color
          FileWrite $0 "$(silentDowngrades)"
        ${EndIf}
      ${Else}
        MessageBox MB_ICONSTOP "$(silentDowngrades)"
      ${EndIf}
      ; AURORA: nel registro e nel rapporto
      StrCpy $AuroraMotivo "versione piu' vecchia di quella installata, rifiutata in modo silenzioso o passivo"
      StrCpy $AuroraCausa "versione"
      Call AuroraFallita
      Abort
    ${EndIf}
  ${EndIf}
  !endif
FunctionEnd

Section WebView2
  ; AURORA: il registro dell'installazione si apre qui, quando l'installazione parte davvero
  Call AuroraApriRegistro
  StrCpy $AuroraPasso "WebView2"
  ; Check if Webview2 is already installed and skip this section
  ${If} ${RunningX64}
    ReadRegStr $4 HKLM "SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\${WEBVIEW2APPGUID}" "pv"
  ${Else}
    ReadRegStr $4 HKLM "SOFTWARE\Microsoft\EdgeUpdate\Clients\${WEBVIEW2APPGUID}" "pv"
  ${EndIf}
  ${If} $4 == ""
    ReadRegStr $4 HKCU "SOFTWARE\Microsoft\EdgeUpdate\Clients\${WEBVIEW2APPGUID}" "pv"
  ${EndIf}

  ${If} $4 == ""
    !insertmacro AuroraScrivi "WebView2: non installato"
    ; AURORA: aggiornando non lo si installa, ma Aurora senza WebView2 non apre la finestra
    ${If} $UpdateMode = 1
      !insertmacro AuroraScrivi "AVVISO: WebView2 assente e non installato perche' e' un aggiornamento (/UPDATE)"
    ${EndIf}
    ; Webview2 installation
    ;
    ; Skip if updating
    ${If} $UpdateMode <> 1
      !if "${INSTALLWEBVIEW2MODE}" == "downloadBootstrapper"
        Delete "$TEMP\MicrosoftEdgeWebview2Setup.exe"
        DetailPrint "$(webview2Downloading)"
        NSISdl::download "https://go.microsoft.com/fwlink/p/?LinkId=2124703" "$TEMP\MicrosoftEdgeWebview2Setup.exe"
        Pop $0
        ${If} $0 == "success"
          DetailPrint "$(webview2DownloadSuccess)"
          !insertmacro AuroraScrivi "WebView2: programma di installazione scaricato"
        ${Else}
          DetailPrint "$(webview2DownloadError)"
          StrCpy $AuroraMotivo "WebView2: scaricamento non riuscito ($0)"
          StrCpy $AuroraCausa "webview2"
          Call AuroraFallita
          Abort "$(webview2AbortError)"
        ${EndIf}
        StrCpy $6 "$TEMP\MicrosoftEdgeWebview2Setup.exe"
        Goto install_webview2
      !endif

      !if "${INSTALLWEBVIEW2MODE}" == "embedBootstrapper"
        Delete "$TEMP\MicrosoftEdgeWebview2Setup.exe"
        File "/oname=$TEMP\MicrosoftEdgeWebview2Setup.exe" "${WEBVIEW2BOOTSTRAPPERPATH}"
        DetailPrint "$(installingWebview2)"
        StrCpy $6 "$TEMP\MicrosoftEdgeWebview2Setup.exe"
        Goto install_webview2
      !endif

      !if "${INSTALLWEBVIEW2MODE}" == "offlineInstaller"
        Delete "$TEMP\MicrosoftEdgeWebView2RuntimeInstaller.exe"
        File "/oname=$TEMP\MicrosoftEdgeWebView2RuntimeInstaller.exe" "${WEBVIEW2INSTALLERPATH}"
        DetailPrint "$(installingWebview2)"
        StrCpy $6 "$TEMP\MicrosoftEdgeWebView2RuntimeInstaller.exe"
        Goto install_webview2
      !endif

      Goto webview2_done

      install_webview2:
        DetailPrint "$(installingWebview2)"
        ; $6 holds the path to the webview2 installer
        ExecWait "$6 ${WEBVIEW2INSTALLERARGS} /install" $1
        ${If} $1 = 0
          DetailPrint "$(webview2InstallSuccess)"
          !insertmacro AuroraScrivi "WebView2: installato"
        ${Else}
          DetailPrint "$(webview2InstallError)"
          StrCpy $AuroraMotivo "WebView2: installazione non riuscita (codice di uscita $1)"
          StrCpy $AuroraCausa "webview2"
          Call AuroraFallita
          Abort "$(webview2AbortError)"
        ${EndIf}
      webview2_done:
    ${EndIf}
  ${Else}
    !insertmacro AuroraScrivi "WebView2: presente ($4)"
    !if "${MINIMUMWEBVIEW2VERSION}" != ""
      ${VersionCompare} "${MINIMUMWEBVIEW2VERSION}" "$4" $R0
      ${If} $R0 = 1
        update_webview:
          DetailPrint "$(installingWebview2)"
          ${If} ${RunningX64}
            ReadRegStr $R1 HKLM "SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate" "path"
          ${Else}
            ReadRegStr $R1 HKLM "SOFTWARE\Microsoft\EdgeUpdate" "path"
          ${EndIf}
          ${If} $R1 == ""
            ReadRegStr $R1 HKCU "SOFTWARE\Microsoft\EdgeUpdate" "path"
          ${EndIf}
          ${If} $R1 != ""
            ; Chromium updater docs: https://source.chromium.org/chromium/chromium/src/+/main:docs/updater/user_manual.md
            ; Modified from "HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\Microsoft EdgeWebView\ModifyPath"
            ExecWait `"$R1" /install appguid=${WEBVIEW2APPGUID}&needsadmin=true` $1
            ${If} $1 = 0
              DetailPrint "$(webview2InstallSuccess)"
              !insertmacro AuroraScrivi "WebView2: aggiornato"
            ${Else}
              MessageBox MB_ICONEXCLAMATION|MB_ABORTRETRYIGNORE "$(webview2InstallError)" IDIGNORE ignore IDRETRY update_webview
              ; AURORA: Quit non passa da .onInstFailed
              StrCpy $AuroraMotivo "WebView2: aggiornamento alla versione minima non riuscito (codice di uscita $1), installazione interrotta"
              StrCpy $AuroraCausa "webview2"
              Call AuroraFallita
              Quit
              ignore:
              !insertmacro AuroraScrivi "AVVISO: WebView2 non aggiornato alla versione minima (codice di uscita $1), proseguito"
            ${EndIf}
          ${EndIf}
      ${EndIf}
    !endif
  ${EndIf}
SectionEnd

Section Install
  SetOutPath $INSTDIR

  !ifmacrodef NSIS_HOOK_PREINSTALL
    !insertmacro NSIS_HOOK_PREINSTALL
  !endif

  ; AURORA: il controllo dei programmi che tengono il file e' nel gancio (AuroraControlloProgramma), che non ferma l'installazione

  ; AURORA: un file non copiato (errore di scrittura ignorato) accende la segnalazione d'errore di NSIS: la si guarda dopo la copia
  StrCpy $AuroraPasso "copia dei file"
  !insertmacro AuroraScrivi "copia dei file"
  ClearErrors

  ; Copy main executable
  File "${MAINBINARYSRCPATH}"

  ; Copy resources
  {{#each resources_dirs}}
    CreateDirectory "$INSTDIR\\{{this}}"
  {{/each}}
  {{#each resources}}
    File /a "/oname={{this.[1]}}" "{{no-escape @key}}"
  {{/each}}

  ; Copy external binaries
  {{#each binaries}}
    File /a "/oname={{this}}" "{{no-escape @key}}"
  {{/each}}

  ; AURORA: i file non scritti (bloccati da un programma o dall'antivirus) nel registro
  ${If} ${Errors}
    !insertmacro AuroraScrivi "AVVISO: alcuni file non sono stati copiati (errore di scrittura ignorato)"
  ${EndIf}
  StrCpy $AuroraPasso "registro di Windows, disinstallatore e collegamenti"

  ; Create file associations
  {{#each file_associations as |association| ~}}
    {{#each association.ext as |ext| ~}}
       !insertmacro APP_ASSOCIATE "{{ext}}" "{{or association.name ext}}" "{{association-description association.description ext}}" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\",0" "Open with ${PRODUCTNAME}" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\" $\"%1$\""
    {{/each}}
  {{/each}}

  ; Register deep links
  {{#each deep_link_protocols as |protocol| ~}}
    WriteRegStr SHCTX "Software\Classes\\{{protocol}}" "URL Protocol" ""
    WriteRegStr SHCTX "Software\Classes\\{{protocol}}" "" "URL:${BUNDLEID} protocol"
    WriteRegStr SHCTX "Software\Classes\\{{protocol}}\DefaultIcon" "" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\",0"
    WriteRegStr SHCTX "Software\Classes\\{{protocol}}\shell\open\command" "" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\" $\"%1$\""
  {{/each}}

  ; Create uninstaller
  WriteUninstaller "$INSTDIR\uninstall.exe"

  ; Save $INSTDIR in registry for future installations
  WriteRegStr SHCTX "${MANUPRODUCTKEY}" "" $INSTDIR

  !if "${INSTALLMODE}" == "both"
    ; Save install mode to be selected by default for the next installation such as updating
    ; or when uninstalling
    WriteRegStr SHCTX "${UNINSTKEY}" $MultiUser.InstallMode 1
  !endif

  ; Remove old main binary if it doesn't match new main binary name
  ReadRegStr $OldMainBinaryName SHCTX "${UNINSTKEY}" "MainBinaryName"
  ${If} $OldMainBinaryName != ""
  ${AndIf} $OldMainBinaryName != "${MAINBINARYNAME}.exe"
    Delete "$INSTDIR\$OldMainBinaryName"
  ${EndIf}

  ; Save current MAINBINARYNAME for future updates
  WriteRegStr SHCTX "${UNINSTKEY}" "MainBinaryName" "${MAINBINARYNAME}.exe"

  ; Registry information for add/remove programs
  WriteRegStr SHCTX "${UNINSTKEY}" "DisplayName" "${PRODUCTNAME}"
  WriteRegStr SHCTX "${UNINSTKEY}" "DisplayIcon" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\""
  WriteRegStr SHCTX "${UNINSTKEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr SHCTX "${UNINSTKEY}" "Publisher" "${MANUFACTURER}"
  WriteRegStr SHCTX "${UNINSTKEY}" "InstallLocation" "$\"$INSTDIR$\""
  WriteRegStr SHCTX "${UNINSTKEY}" "UninstallString" "$\"$INSTDIR\uninstall.exe$\""
  WriteRegDWORD SHCTX "${UNINSTKEY}" "NoModify" "1"
  WriteRegDWORD SHCTX "${UNINSTKEY}" "NoRepair" "1"

  ${GetSize} "$INSTDIR" "/M=uninstall.exe /S=0K /G=0" $0 $1 $2
  IntOp $0 $0 + ${ESTIMATEDSIZE}
  IntFmt $0 "0x%08X" $0
  WriteRegDWORD SHCTX "${UNINSTKEY}" "EstimatedSize" "$0"

  !if "${HOMEPAGE}" != ""
    WriteRegStr SHCTX "${UNINSTKEY}" "URLInfoAbout" "${HOMEPAGE}"
    WriteRegStr SHCTX "${UNINSTKEY}" "URLUpdateInfo" "${HOMEPAGE}"
    WriteRegStr SHCTX "${UNINSTKEY}" "HelpLink" "${HOMEPAGE}"
  !endif

  ; Create start menu shortcut
  !insertmacro MUI_STARTMENU_WRITE_BEGIN Application
    Call CreateOrUpdateStartMenuShortcut
  !insertmacro MUI_STARTMENU_WRITE_END

  ; Create desktop shortcut for silent and passive installers
  ; because finish page will be skipped
  ${If} $PassiveMode = 1
  ${OrIf} ${Silent}
    Call CreateOrUpdateDesktopShortcut
  ${EndIf}

  !ifmacrodef NSIS_HOOK_POSTINSTALL
    !insertmacro NSIS_HOOK_POSTINSTALL
  !endif

  ; Auto close this page for passive mode
  ${If} $PassiveMode = 1
    SetAutoClose true
  ${EndIf}
SectionEnd

; AURORA: un Abort in una sezione (WebView2, Aurora che non si chiude, file non scritti): registro e rapporto anonimo
Function .onInstFailed
  Call AuroraFallita
FunctionEnd

Function .onInstSuccess
  ; Check for `/R` flag only in silent and passive installers because
  ; GUI installer has a toggle for the user to (re)start the app
  ${If} $PassiveMode = 1
  ${OrIf} ${Silent}
    ${GetOptions} $CMDLINE "/R" $R0
    ${IfNot} ${Errors}
      ${GetOptions} $CMDLINE "/ARGS" $R0
      nsis_tauri_utils::RunAsUser "$INSTDIR\${MAINBINARYNAME}.exe" "$R0"
    ${EndIf}
  ${EndIf}
FunctionEnd

Function un.onInit
  !insertmacro SetContext

  !if "${INSTALLMODE}" == "both"
    !insertmacro MULTIUSER_UNINIT
  !endif

  !insertmacro MUI_UNGETLANGUAGE

  ${GetOptions} $CMDLINE "/P" $PassiveMode
  ${IfNot} ${Errors}
    StrCpy $PassiveMode 1
  ${EndIf}

  ${GetOptions} $CMDLINE "/UPDATE" $UpdateMode
  ${IfNot} ${Errors}
    StrCpy $UpdateMode 1
  ${EndIf}
FunctionEnd

Section Uninstall

  !ifmacrodef NSIS_HOOK_PREUNINSTALL
    !insertmacro NSIS_HOOK_PREUNINSTALL
  !endif

  !insertmacro CheckIfAppIsRunning "$INSTDIR\${MAINBINARYNAME}.exe" "${PRODUCTNAME}"

  ; Delete the app directory and its content from disk
  ; Copy main executable
  Delete "$INSTDIR\${MAINBINARYNAME}.exe"

  ; Delete resources
  {{#each resources}}
    Delete "$INSTDIR\\{{this.[1]}}"
  {{/each}}

  ; Delete external binaries
  {{#each binaries}}
    Delete "$INSTDIR\\{{this}}"
  {{/each}}

  ; Delete app associations
  {{#each file_associations as |association| ~}}
    {{#each association.ext as |ext| ~}}
      !insertmacro APP_UNASSOCIATE "{{ext}}" "{{or association.name ext}}"
    {{/each}}
  {{/each}}

  ; Delete deep links
  {{#each deep_link_protocols as |protocol| ~}}
    ReadRegStr $R7 SHCTX "Software\Classes\\{{protocol}}\shell\open\command" ""
    ${If} $R7 == "$\"$INSTDIR\${MAINBINARYNAME}.exe$\" $\"%1$\""
      DeleteRegKey SHCTX "Software\Classes\\{{protocol}}"
    ${EndIf}
  {{/each}}


  ; Delete uninstaller
  Delete "$INSTDIR\uninstall.exe"

  {{#each resources_ancestors}}
  RMDir /REBOOTOK "$INSTDIR\\{{this}}"
  {{/each}}
  RMDir "$INSTDIR"

  ; Remove shortcuts if not updating
  ${If} $UpdateMode <> 1
    !insertmacro DeleteAppUserModelId

    ; Remove start menu shortcut
    !insertmacro MUI_STARTMENU_GETFOLDER Application $AppStartMenuFolder
    !insertmacro IsShortcutTarget "$SMPROGRAMS\$AppStartMenuFolder\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
    Pop $0
    ${If} $0 = 1
      !insertmacro UnpinShortcut "$SMPROGRAMS\$AppStartMenuFolder\${PRODUCTNAME}.lnk"
      Delete "$SMPROGRAMS\$AppStartMenuFolder\${PRODUCTNAME}.lnk"
      RMDir "$SMPROGRAMS\$AppStartMenuFolder"
    ${EndIf}
    !insertmacro IsShortcutTarget "$SMPROGRAMS\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
    Pop $0
    ${If} $0 = 1
      !insertmacro UnpinShortcut "$SMPROGRAMS\${PRODUCTNAME}.lnk"
      Delete "$SMPROGRAMS\${PRODUCTNAME}.lnk"
    ${EndIf}

    ; Remove desktop shortcuts
    !insertmacro IsShortcutTarget "$DESKTOP\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
    Pop $0
    ${If} $0 = 1
      !insertmacro UnpinShortcut "$DESKTOP\${PRODUCTNAME}.lnk"
      Delete "$DESKTOP\${PRODUCTNAME}.lnk"
    ${EndIf}
  ${EndIf}

  ; Remove registry information for add/remove programs
  !if "${INSTALLMODE}" == "both"
    DeleteRegKey SHCTX "${UNINSTKEY}"
  !else if "${INSTALLMODE}" == "perMachine"
    DeleteRegKey HKLM "${UNINSTKEY}"
  !else
    DeleteRegKey HKCU "${UNINSTKEY}"
  !endif

  ; Removes the Autostart entry for ${PRODUCTNAME} from the HKCU Run key if it exists.
  ; This ensures the program does not launch automatically after uninstallation if it exists.
  ; If it doesn't exist, it does nothing.
  ; We do this when not updating (to preserve the registry value on updates)
  ${If} $UpdateMode <> 1
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${PRODUCTNAME}"
  ${EndIf}

  ; Delete app data if the checkbox is selected
  ; and if not updating
  ${If} $DeleteAppDataCheckboxState = 1
  ${AndIf} $UpdateMode <> 1
    ; Clear the install location $INSTDIR from registry
    DeleteRegKey SHCTX "${MANUPRODUCTKEY}"
    DeleteRegKey /ifempty SHCTX "${MANUKEY}"

    ; Clear the install language from registry
    DeleteRegValue HKCU "${MANUPRODUCTKEY}" "Installer Language"
    DeleteRegKey /ifempty HKCU "${MANUPRODUCTKEY}"
    DeleteRegKey /ifempty HKCU "${MANUKEY}"

    SetShellVarContext current
    RmDir /r "$APPDATA\${BUNDLEID}"
    RmDir /r "$LOCALAPPDATA\${BUNDLEID}"
  ${EndIf}

  !ifmacrodef NSIS_HOOK_POSTUNINSTALL
    !insertmacro NSIS_HOOK_POSTUNINSTALL
  !endif

  ; Auto close if passive mode or updating
  ${If} $PassiveMode = 1
  ${OrIf} $UpdateMode = 1
    SetAutoClose true
  ${EndIf}
SectionEnd

Function RestorePreviousInstallLocation
  ReadRegStr $4 SHCTX "${MANUPRODUCTKEY}" ""
  StrCmp $4 "" +2 0
    StrCpy $INSTDIR $4
FunctionEnd

Function Skip
  Abort
FunctionEnd

Function SkipIfPassive
  ${IfThen} $PassiveMode = 1  ${|} Abort ${|}
FunctionEnd
Function un.SkipIfPassive
  ${IfThen} $PassiveMode = 1  ${|} Abort ${|}
FunctionEnd

Function CreateOrUpdateStartMenuShortcut
  ; We used to use product name as MAINBINARYNAME
  ; migrate old shortcuts to target the new MAINBINARYNAME
  StrCpy $R0 0

  !insertmacro IsShortcutTarget "$SMPROGRAMS\$AppStartMenuFolder\${PRODUCTNAME}.lnk" "$INSTDIR\$OldMainBinaryName"
  Pop $0
  ${If} $0 = 1
    !insertmacro SetShortcutTarget "$SMPROGRAMS\$AppStartMenuFolder\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
    StrCpy $R0 1
  ${EndIf}

  !insertmacro IsShortcutTarget "$SMPROGRAMS\${PRODUCTNAME}.lnk" "$INSTDIR\$OldMainBinaryName"
  Pop $0
  ${If} $0 = 1
    !insertmacro SetShortcutTarget "$SMPROGRAMS\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
    StrCpy $R0 1
  ${EndIf}

  ${If} $R0 = 1
    Return
  ${EndIf}

  ; Skip creating shortcut if in update mode or no shortcut mode
  ; but always create if migrating from wix
  ${If} $WixMode = 0
    ${If} $UpdateMode = 1
    ${OrIf} $NoShortcutMode = 1
      Return
    ${EndIf}
  ${EndIf}

  !if "${STARTMENUFOLDER}" != ""
    CreateDirectory "$SMPROGRAMS\$AppStartMenuFolder"
    CreateShortcut "$SMPROGRAMS\$AppStartMenuFolder\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
    !insertmacro SetLnkAppUserModelId "$SMPROGRAMS\$AppStartMenuFolder\${PRODUCTNAME}.lnk"
  !else
    CreateShortcut "$SMPROGRAMS\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
    !insertmacro SetLnkAppUserModelId "$SMPROGRAMS\${PRODUCTNAME}.lnk"
  !endif
FunctionEnd

Function CreateOrUpdateDesktopShortcut
  ; We used to use product name as MAINBINARYNAME
  ; migrate old shortcuts to target the new MAINBINARYNAME
  !insertmacro IsShortcutTarget "$DESKTOP\${PRODUCTNAME}.lnk" "$INSTDIR\$OldMainBinaryName"
  Pop $0
  ${If} $0 = 1
    !insertmacro SetShortcutTarget "$DESKTOP\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
    Return
  ${EndIf}

  ; Skip creating shortcut if in update mode or no shortcut mode
  ; but always create if migrating from wix
  ${If} $WixMode = 0
    ${If} $UpdateMode = 1
    ${OrIf} $NoShortcutMode = 1
      Return
    ${EndIf}
  ${EndIf}

  CreateShortcut "$DESKTOP\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
  !insertmacro SetLnkAppUserModelId "$DESKTOP\${PRODUCTNAME}.lnk"
FunctionEnd
