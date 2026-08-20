export function DeviceScopeNotice() {
    return (
        <aside className="device-scope-notice" aria-label="当前设备数据说明">
            <strong>仅影响此 Windows 设备</strong>
            <p>
                配置和演示体验保存在当前设备。当前 MVP
                不提供云备份或跨设备同步。
            </p>
        </aside>
    );
}
