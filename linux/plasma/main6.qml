import QtQuick
import org.kde.plasma.plasmoid

WallpaperItem {
    id: root
    FrameView {
        anchors.fill: parent
        endpoint: root.configuration.SourceUrl
        fps: root.configuration.Fps
    }
}
