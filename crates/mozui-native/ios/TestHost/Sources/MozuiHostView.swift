import QuartzCore
import UIKit

final class MozuiHostView: UIView {
    override class var layerClass: AnyClass { CAMetalLayer.self }

    weak var hostViewController: UIViewController?

    private let demo: OpaquePointer
    private var displayLink: CADisplayLink?

    init(demo: OpaquePointer) {
        self.demo = demo
        super.init(frame: .zero)
        backgroundColor = .black
        contentScaleFactor = UIScreen.main.scale
        isMultipleTouchEnabled = false
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        nil
    }

    override func didMoveToWindow() {
        super.didMoveToWindow()

        if window != nil {
            startDisplayLink()
            attachIfPossible()
            pushMetrics()
        } else {
            stopDisplayLink()
            mozui_ios_demo_detach_view(demo)
        }
    }

    override func layoutSubviews() {
        super.layoutSubviews()
        if let metalLayer = layer as? CAMetalLayer {
            metalLayer.frame = bounds
            metalLayer.drawableSize = CGSize(
                width: bounds.width * contentScaleFactor,
                height: bounds.height * contentScaleFactor
            )
        }
        pushMetrics()
    }

    override func safeAreaInsetsDidChange() {
        super.safeAreaInsetsDidChange()
        pushMetrics()
    }

    override func traitCollectionDidChange(_ previousTraitCollection: UITraitCollection?) {
        super.traitCollectionDidChange(previousTraitCollection)
        pushMetrics()
    }

    override func touchesBegan(_ touches: Set<UITouch>, with event: UIEvent?) {
        super.touchesBegan(touches, with: event)
        forwardTouch(touches, phase: Int32(MOZUI_IOS_TOUCH_BEGAN))
    }

    override func touchesMoved(_ touches: Set<UITouch>, with event: UIEvent?) {
        super.touchesMoved(touches, with: event)
        forwardTouch(touches, phase: Int32(MOZUI_IOS_TOUCH_MOVED))
    }

    override func touchesEnded(_ touches: Set<UITouch>, with event: UIEvent?) {
        super.touchesEnded(touches, with: event)
        forwardTouch(touches, phase: Int32(MOZUI_IOS_TOUCH_ENDED))
    }

    override func touchesCancelled(_ touches: Set<UITouch>, with event: UIEvent?) {
        super.touchesCancelled(touches, with: event)
        forwardTouch(touches, phase: Int32(MOZUI_IOS_TOUCH_CANCELLED))
    }

    private func attachIfPossible() {
        guard let hostViewController else { return }

        let didAttach = mozui_ios_demo_attach_view(
            demo,
            Unmanaged.passUnretained(self).toOpaque(),
            Unmanaged.passUnretained(hostViewController).toOpaque()
        )
        if !didAttach {
            logLastError(prefix: "attach_view failed")
        }
    }

    private func pushMetrics() {
        guard window != nil else { return }

        let visibleBounds = bounds.inset(by: safeAreaInsets)
        let appearance: Int32 =
            switch traitCollection.userInterfaceStyle {
            case .dark:
                Int32(MOZUI_IOS_APPEARANCE_DARK)
            default:
                Int32(MOZUI_IOS_APPEARANCE_LIGHT)
            }

        let metrics = MozuiIosHostMetrics(
            bounds_width: Float(bounds.width),
            bounds_height: Float(bounds.height),
            visible_x: Float(visibleBounds.origin.x),
            visible_y: Float(visibleBounds.origin.y),
            visible_width: Float(visibleBounds.width),
            visible_height: Float(visibleBounds.height),
            scale_factor: Float(window?.screen.scale ?? UIScreen.main.scale),
            appearance: appearance
        )

        mozui_ios_demo_update_metrics(demo, metrics)
    }

    private func logLastError(prefix: String) {
        guard let errorPtr = mozui_ios_demo_last_error(demo) else { return }
        print("[MozuiIOSHost] \(prefix): \(String(cString: errorPtr))")
    }

    private func forwardTouch(_ touches: Set<UITouch>, phase: Int32) {
        guard let touch = touches.first else { return }
        let position = touch.location(in: self)
        mozui_ios_demo_handle_touch(demo, Float(position.x), Float(position.y), phase)
    }

    private func startDisplayLink() {
        guard displayLink == nil else { return }
        let link = CADisplayLink(target: self, selector: #selector(stepDisplayLink))
        link.add(to: .main, forMode: .common)
        displayLink = link
    }

    private func stopDisplayLink() {
        displayLink?.invalidate()
        displayLink = nil
    }

    @objc
    private func stepDisplayLink() {
        mozui_ios_demo_request_frame(demo)
    }
}
