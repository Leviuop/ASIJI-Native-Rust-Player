import QtQuick 2.15

Rectangle {
    id: view
    color: "black"
    property string endpoint: ""
    property int fps: 30
    property var pendingImage: null
    function requestFrame() {
        if (pendingImage || !/^http:\/\/127\.0\.0\.1:[0-9]+\/[a-f0-9]{64}$/.test(endpoint))
            return;
        pendingImage = first.visible ? second : first;
        pendingImage.source = endpoint + "/frame.bmp?t=" + Date.now();
    }
    function loaded(item) {
        if (item !== pendingImage)
            return;
        if (item.status === Image.Ready) {
            first.visible = item === first;
            second.visible = item === second;
            pendingImage = null;
        } else if (item.status === Image.Error) {
            pendingImage = null;
        }
    }
    onEndpointChanged: {
        pendingImage = null;
        first.visible = false;
        second.visible = false;
        requestFrame();
    }
    Image {
        id: first
        anchors.fill: parent
        visible: false
        cache: false
        asynchronous: true
        smooth: false
        fillMode: Image.PreserveAspectFit
        onStatusChanged: view.loaded(first)
    }
    Image {
        id: second
        anchors.fill: parent
        visible: false
        cache: false
        asynchronous: true
        smooth: false
        fillMode: Image.PreserveAspectFit
        onStatusChanged: view.loaded(second)
    }
    Timer {
        interval: Math.max(16, Math.round(1000 / Math.max(10, view.fps)))
        running: view.visible && view.endpoint.length > 0
        repeat: true
        onTriggered: view.requestFrame()
    }
}
