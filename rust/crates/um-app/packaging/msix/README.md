# Microsoft Store용 MSIX

`.github/workflows/rust-package.yml`을 수동 실행하거나 `rust-v*` 태그를 푸시하면 x64·ARM64 MSIX와 두 아키텍처를 묶은 MSIX 번들을 만든다. MSIX는 `production,store` 기능으로 빌드하며 서명하지 않는다. 태그 실행 결과는 GitHub의 비공개 초안 릴리스(prerelease)에 첨부된다. NSIS 설치 파일과 포터블 zip은 직접 배포판이다.

## Partner Center 값

앱 이름을 예약한 뒤 제품 ID 화면에서 다음 값을 확인하여 `AppxManifest.xml`에 반영한다.

- `Identity Name="WWhaleFe.UsageMeter"`: 패키지/ID/이름(Package/Identity/Name).
- `Identity Publisher="CN=PLACEHOLDER-PARTNER-CENTER"`: 패키지/ID/게시자(Package/Identity/Publisher). 공백과 구두점까지 일치해야 한다.
- `PublisherDisplayName`: Partner Center에 등록한 게시자 표시 이름. 현재 값은 `WWhaleFe`.

`__VERSION__`과 `__ARCH__`는 템플릿에 남겨 둔다. CI가 Cargo 워크스페이스 버전 `2.0.0-beta.1`을 MSIX 버전 `2.0.0.0`으로 바꾸고 `x64` 또는 `arm64`를 넣는다. Store 제출용 마지막 자리는 0이다. 같은 숫자 버전의 베타 접미사만 바꾸면 MSIX 버전은 같으므로 업데이트 검사에는 숫자 버전도 올려야 한다.

## Windows 로컬 설치

Windows 10 22H2 이상에서 개발자 모드를 켠다. WebView2 Runtime도 필요하다(Windows 11에는 기본 포함). 개발자 모드만으로 일반적인 `Add-AppxPackage` 서명 검사를 생략할 수 없으므로, CI에서 받은 해당 아키텍처의 MSIX 사본에 테스트 서명한다. Windows SDK를 설치하고 관리자 PowerShell에서 다음 예를 실행한다. `signtool.exe`는 SDK의 `Windows Kits\10\bin\<버전>\x64` 경로를 사용한다.

```powershell
# 실제 패키지의 Publisher 값과 정확히 일치시킨다.
$publisher = 'CN=PLACEHOLDER-PARTNER-CENTER'
$package = '.\UsageMeter-2.0.0-beta.1-x64.msix'
$cert = New-SelfSignedCertificate -Type Custom -Subject $publisher -KeyUsage DigitalSignature -CertStoreLocation 'Cert:\CurrentUser\My' -TextExtension @('2.5.29.37={text}1.3.6.1.5.5.7.3.3', '2.5.29.19={text}')
Export-Certificate -Cert $cert -FilePath .\UsageMeter-test.cer
Import-Certificate -FilePath .\UsageMeter-test.cer -CertStoreLocation 'Cert:\LocalMachine\TrustedPeople'
signtool.exe sign /fd SHA256 /s My /sha1 $cert.Thumbprint $package
Add-AppxPackage -Path $package
```

테스트 인증서는 점검 기기에서만 신뢰한다. 설치 후 로그인·테두리·분석·제거와 Windows 설정의 시작 앱을 확인한다. `UsageMeterStartup`은 기본 비활성 상태이며, 매니페스트 선언 외에 앱에서 StartupTask를 켜고 끄는 연동은 별도 실기 점검 대상이다. 테스트 서명한 사본은 Store에 제출하지 않는다.

## Store 제출 순서

1. Partner Center 개발자 등록과 앱 이름 예약 후 위 ID·게시자 값을 반영한다.
2. 워크플로를 실행하고 Windows x64·ARM64에서 설치·동작을 점검한다. Windows App Certification Kit 검사도 실행한다.
3. CI의 서명 없는 `UsageMeter-<버전>.msixbundle`을 Partner Center 패키지 항목에 업로드하고 사전 검증 결과를 확인한다. 배포 시 Microsoft가 서명한다.
4. 한국어·영어·일본어 설명, 스크린샷, 개인정보 처리방침 URL, 연령 등급, 전체 신뢰 기능과 앱 내 제3자 로그인에 대한 심사 설명을 작성한다.
5. 제출 내용을 검토한 뒤 Partner Center에서 최종 제출한다. GitHub 초안 릴리스는 별도로 검토 후 공개한다.

참고: [MSIX 패키지 생성](https://learn.microsoft.com/en-us/windows/msix/package/create-app-package-with-makeappx-tool), [테스트 인증서 생성](https://learn.microsoft.com/en-us/windows/msix/package/create-certificate-package-signing).
