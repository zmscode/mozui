import QuartzCore
import UIKit

final class MozuiHostView: UIView, UIKeyInput, UIGestureRecognizerDelegate {
	override class var layerClass: AnyClass { CAMetalLayer.self }

	weak var hostViewController: UIViewController?

	private static let idleFrameBudget = 2
	private static let interactionFrameBudget = 60
	private static let textInputFrameBudget = 30
	private static let decelerationRate: CGFloat = 0.92
	private static let velocityThreshold: CGFloat = 4.0

	private let demo: OpaquePointer
	private var displayLink: CADisplayLink?
	private lazy var panRecognizer: UIPanGestureRecognizer = {
		let recognizer = UIPanGestureRecognizer(target: self, action: #selector(handlePan(_:)))
		recognizer.cancelsTouchesInView = false
		recognizer.delegate = self
		return recognizer
	}()
	private var pendingDisplayFrames = 0
	private var inertialScrollVelocity = CGPoint.zero
	private var inertialScrollPosition = CGPoint.zero
	private var wantsTextInputFocus = false

	init(demo: OpaquePointer) {
		self.demo = demo
		super.init(frame: .zero)
		backgroundColor = .black
		contentScaleFactor = UIScreen.main.scale
		isMultipleTouchEnabled = false
		addGestureRecognizer(panRecognizer)
	}

	@available(*, unavailable)
	required init?(coder: NSCoder) {
		nil
	}

	override var canBecomeFirstResponder: Bool { true }

	var hasText: Bool { true }

	func insertText(_ text: String) {
		_ = text.withCString { cString in
			mozui_ios_demo_insert_text(demo, cString)
		}
		requestDisplayFrames(Self.textInputFrameBudget)
		syncTextInputFocus()
	}

	func deleteBackward() {
		_ = mozui_ios_demo_delete_backward(demo)
		requestDisplayFrames(Self.textInputFrameBudget)
		syncTextInputFocus()
	}

	override func didMoveToWindow() {
		super.didMoveToWindow()

		if window != nil {
			startDisplayLink()
			attachIfPossible()
			pushMetrics()
			requestDisplayFrames(Self.idleFrameBudget)
		} else {
			wantsTextInputFocus = false
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
		requestDisplayFrames(Self.idleFrameBudget)
	}

	override func safeAreaInsetsDidChange() {
		super.safeAreaInsetsDidChange()
		pushMetrics()
		requestDisplayFrames(Self.idleFrameBudget)
	}

	override func traitCollectionDidChange(_ previousTraitCollection: UITraitCollection?) {
		super.traitCollectionDidChange(previousTraitCollection)
		pushMetrics()
		requestDisplayFrames(Self.idleFrameBudget)
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
		let appearance: Int32
		switch traitCollection.userInterfaceStyle {
		case .dark:
			appearance = Int32(MOZUI_IOS_APPEARANCE_DARK)
		default:
			appearance = Int32(MOZUI_IOS_APPEARANCE_LIGHT)
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
		requestDisplayFrames(Self.interactionFrameBudget)
		syncTextInputFocus()
	}

	private func startDisplayLink() {
		guard displayLink == nil else { return }
		let link = CADisplayLink(target: self, selector: #selector(stepDisplayLink))
		link.add(to: .main, forMode: .common)
		link.isPaused = true
		displayLink = link
	}

	private func stopDisplayLink() {
		displayLink?.invalidate()
		displayLink = nil
	}

	@objc
	private func stepDisplayLink() {
		if inertialScrollVelocity != .zero {
			mozui_ios_demo_handle_scroll(
				demo,
				Float(inertialScrollPosition.x),
				Float(inertialScrollPosition.y),
				Float(inertialScrollVelocity.x),
				Float(inertialScrollVelocity.y),
				1
			)
			inertialScrollVelocity.x *= Self.decelerationRate
			inertialScrollVelocity.y *= Self.decelerationRate

			if abs(inertialScrollVelocity.x) < Self.velocityThreshold {
				inertialScrollVelocity.x = 0
			}
			if abs(inertialScrollVelocity.y) < Self.velocityThreshold {
				inertialScrollVelocity.y = 0
			}
		}

		guard pendingDisplayFrames > 0 || inertialScrollVelocity != .zero else {
			displayLink?.isPaused = true
			return
		}

			if pendingDisplayFrames > 0 {
				pendingDisplayFrames -= 1
			}
			mozui_ios_demo_request_frame(demo)

			if pendingDisplayFrames == 0 && inertialScrollVelocity == .zero {
				displayLink?.isPaused = true
			}
	}

	private func syncTextInputFocus() {
		let wantsTextInput = mozui_ios_demo_accepts_text_input(demo)
		guard wantsTextInput != wantsTextInputFocus else { return }
		wantsTextInputFocus = wantsTextInput

		if wantsTextInput {
			if !isFirstResponder {
				_ = becomeFirstResponder()
			}
		} else if isFirstResponder {
			_ = resignFirstResponder()
		}
	}

	private func requestDisplayFrames(_ frameCount: Int) {
		pendingDisplayFrames = max(pendingDisplayFrames, frameCount)
		displayLink?.isPaused = false
	}

	@objc
	private func handlePan(_ recognizer: UIPanGestureRecognizer) {
		let position = recognizer.location(in: self)
		let translation = recognizer.translation(in: self)
		recognizer.setTranslation(.zero, in: self)
		inertialScrollPosition = position

		let phase: Int32
		switch recognizer.state {
		case .began:
			inertialScrollVelocity = .zero
			phase = 0
		case .ended, .cancelled, .failed:
			let velocity = recognizer.velocity(in: self)
			inertialScrollVelocity = CGPoint(x: velocity.x / 60.0, y: velocity.y / 60.0)
			requestDisplayFrames(Self.interactionFrameBudget)
			phase = 2
		default:
			phase = 1
		}

		mozui_ios_demo_handle_scroll(
			demo,
			Float(position.x),
			Float(position.y),
			Float(translation.x),
			Float(translation.y),
			phase
		)
		requestDisplayFrames(Self.interactionFrameBudget)
	}

	func gestureRecognizer(
		_ gestureRecognizer: UIGestureRecognizer,
		shouldRecognizeSimultaneouslyWith otherGestureRecognizer: UIGestureRecognizer
	) -> Bool {
		true
	}
}
