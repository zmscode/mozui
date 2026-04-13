import UIKit

final class MozuiHostViewController: UIViewController {
    private let demo: OpaquePointer

    init(demo: OpaquePointer) {
        self.demo = demo
        super.init(nibName: nil, bundle: nil)
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        nil
    }

    override func loadView() {
        let hostView = MozuiHostView(demo: demo)
        hostView.hostViewController = self
        view = hostView
    }
}
