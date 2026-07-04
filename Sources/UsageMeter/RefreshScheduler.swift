import Foundation
import Combine

/// 설정한 주기마다 모든 provider의 사용량을 자동 갱신한다.
/// 주기 설정이 바뀌면 타이머를 다시 건다(값이 실제로 달라졌을 때만).
@MainActor
final class RefreshScheduler {
    private let settings: OverlaySettings
    private let manager: ProviderManager
    private var timer: Timer?
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
