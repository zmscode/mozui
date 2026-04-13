import UIKit

final class SceneDelegate: UIResponder, UIWindowSceneDelegate {
    var window: UIWindow?

    private var demo: OpaquePointer?

    func scene(
        _ scene: UIScene,
        willConnectTo session: UISceneSession,
        options connectionOptions: UIScene.ConnectionOptions
    ) {
        guard let windowScene = scene as? UIWindowScene else { return }
        guard let demo = mozui_ios_demo_new() else {
            assertionFailure("mozui_ios_demo_new returned nil")
            return
        }

        self.demo = demo
        mozui_ios_demo_update_metrics(demo, initialMetrics(for: windowScene))

        let window = UIWindow(windowScene: windowScene)
        window.rootViewController = MozuiHostViewController(demo: demo)
        window.makeKeyAndVisible()
        self.window = window
    }

    func sceneWillEnterForeground(_ scene: UIScene) {
        guard let demo else { return }
        mozui_ios_demo_enter_foreground(demo)
    }

    func sceneDidEnterBackground(_ scene: UIScene) {
        guard let demo else { return }
        mozui_ios_demo_enter_background(demo)
    }

    func sceneDidDisconnect(_ scene: UIScene) {
        guard let demo else { return }
        mozui_ios_demo_free(demo)
        self.demo = nil
        window = nil
    }

    private func initialMetrics(for windowScene: UIWindowScene) -> MozuiIosHostMetrics {
        let bounds = windowScene.screen.bounds
        let appearance: Int32 =
            switch windowScene.traitCollection.userInterfaceStyle {
            case .dark:
                Int32(MOZUI_IOS_APPEARANCE_DARK)
            default:
                Int32(MOZUI_IOS_APPEARANCE_LIGHT)
            }

        return MozuiIosHostMetrics(
            bounds_width: Float(bounds.width),
            bounds_height: Float(bounds.height),
            visible_x: 0,
            visible_y: 0,
            visible_width: Float(bounds.width),
            visible_height: Float(bounds.height),
            scale_factor: Float(windowScene.screen.scale),
            appearance: appearance
        )
    }
}
