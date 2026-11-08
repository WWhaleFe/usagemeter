import Foundation
import Combine

/// 설정한 주기마다 모든 provider의 사용량을 자동 갱신하고,
/// 로컬 `agy` 활동(FSEvents) 감지 시 즉시(1~2초 내) Gemini/Antigravity 사용량을 갱신한다.
@MainActor
final class RefreshScheduler {
    private let settings: OverlaySettings
    private let manager: ProviderManager
    private var timer: Timer?
    private var fastTimer: Timer?
    private var agyWatcher: AgyActivityWatcher?
    private var lastMinutes: Int = -1
    private var cancellable: AnyCancellable?

    init(settings: OverlaySettings, manager: ProviderManager) {
        self.settings = settings
        self.manager = manager
        // 주기 관련 설정이 바뀌면 다시 스케줄(디바운스).
        cancellable = settings.objectWillChange
            .debounce(for: .milliseconds(300), scheduler: RunLoop.main)
            .sink { [weak self] in self?.reschedule() }
        reschedule()

        // 1) ~/.gemini/antigravity-cli 대화/히스토리 변경 감지 시 즉시 갱신.
        agyWatcher = AgyActivityWatcher { [weak self] in
            self?.manager.refresh("gemini")
        }

        // 2) 최근 agy 활동이 있을 때는 25초 주기로 보조 폴링을 병행해 서버 반영 지연을 놓치지 않는다.
        fastTimer = Timer.scheduledTimer(withTimeInterval: 25, repeats: true) { [weak self] _ in
            Task { @MainActor [weak self] in
                guard let self else { return }
                if await AgyQuotaFetcher.shared.isRecentlyActive(within: 600) {
                    self.manager.refresh("gemini")
                }
            }
        }
    }

    private func reschedule() {
        let minutes = settings.effectiveRefreshMinutes
        guard minutes != lastMinutes else { return }   // 주기가 실제로 바뀐 경우만
        lastMinutes = minutes
        timer?.invalidate()
        timer = Timer.scheduledTimer(withTimeInterval: TimeInterval(minutes * 60), repeats: true) { [weak self] _ in
            Task { @MainActor in self?.manager.refreshAll() }
        }
    }
}
