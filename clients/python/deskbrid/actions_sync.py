from __future__ import annotations

from typing import Any

from .models import ClipboardContent, DaemonInfo, MonitorInfo, WindowInfo


class SyncActionsMixin:
    def type_text(self, text: str) -> None:
        self._loop.submit(self._client.type_text(text)).result()

    def send_keys(self, keys: list[str]) -> None:
        self._loop.submit(self._client.send_keys(keys)).result()

    def mouse_click(self, x: int, y: int, button: str = "left") -> None:
        self._loop.submit(self._client.mouse_click(x=x, y=y, button=button)).result()

    def clipboard_read(self) -> ClipboardContent:
        return self._loop.submit(self._client.clipboard_read()).result()

    def clipboard_write(self, text: str) -> None:
        self._loop.submit(self._client.clipboard_write(text)).result()

    def screenshot(self, monitor: int | None = None) -> str:
        result = self._loop.submit(self._client.screenshot(monitor=monitor)).result()
        return result.path

    def screenshot_ocr(
        self,
        path: str | None = None,
        language: str | None = None,
        psm: int | None = None,
        bounding_boxes: bool = False,
        monitor: int | None = None,
        region: dict[str, int] | None = None,
        window_id: str | None = None,
    ) -> dict[str, Any]:
        return self._loop.submit(
            self._client.screenshot_ocr(
                path=path,
                language=language,
                psm=psm,
                bounding_boxes=bounding_boxes,
                monitor=monitor,
                region=region,
                window_id=window_id,
            )
        ).result()

    def screenshot_diff(
        self,
        before_path: str,
        after_path: str | None = None,
        tolerance: int | None = None,
        diff_path: str | None = None,
        save_diff: bool = False,
        monitor: int | None = None,
        region: dict[str, int] | None = None,
        window_id: str | None = None,
    ) -> dict[str, Any]:
        return self._loop.submit(
            self._client.screenshot_diff(
                before_path=before_path,
                after_path=after_path,
                tolerance=tolerance,
                diff_path=diff_path,
                save_diff=save_diff,
                monitor=monitor,
                region=region,
                window_id=window_id,
            )
        ).result()

    def notify(self, title: str, body: str = "", urgency: str = "normal") -> int:
        return self._loop.submit(self._client.notify(title, body, urgency)).result()

    def list_windows(self) -> list[WindowInfo]:
        return self._loop.submit(self._client.list_windows()).result()

    def focus_window(
        self,
        *,
        app_id: str | None = None,
        title: str | None = None,
        exact: bool = False,
    ) -> None:
        self._loop.submit(
            self._client.focus_window(app_id=app_id, title=title, exact=exact)
        ).result()

    def activate_or_launch(
        self,
        app_id: str,
        command: list[str] | None = None,
        workdir: str | None = None,
        env: dict[str, str] | None = None,
    ) -> dict[str, Any]:
        return self._loop.submit(
            self._client.activate_or_launch(
                app_id=app_id,
                command=command,
                workdir=workdir,
                env=env,
            )
        ).result()

    def list_layout_profiles(self) -> list[dict[str, Any]]:
        return self._loop.submit(self._client.list_layout_profiles()).result()

    def save_layout_profile(self, name: str, overwrite: bool = False) -> dict[str, Any]:
        return self._loop.submit(
            self._client.save_layout_profile(name=name, overwrite=overwrite)
        ).result()

    def get_layout_profile(self, name: str) -> dict[str, Any]:
        return self._loop.submit(self._client.get_layout_profile(name)).result()

    def delete_layout_profile(self, name: str) -> dict[str, Any]:
        return self._loop.submit(self._client.delete_layout_profile(name)).result()

    def restore_layout_profile(self, name: str) -> dict[str, Any]:
        return self._loop.submit(self._client.restore_layout_profile(name)).result()

    def list_displays(self) -> list[MonitorInfo]:
        return self._loop.submit(self._client.list_displays()).result()

    def set_primary_monitor(self, output: str) -> dict[str, Any]:
        return self._loop.submit(self._client.set_primary_monitor(output)).result()

    def set_monitor_resolution(
        self,
        output: str,
        width: int,
        height: int,
        refresh_rate: float | None = None,
    ) -> dict[str, Any]:
        return self._loop.submit(
            self._client.set_monitor_resolution(
                output=output,
                width=width,
                height=height,
                refresh_rate=refresh_rate,
            )
        ).result()

    def set_monitor_scale(self, output: str, scale: float) -> dict[str, Any]:
        return self._loop.submit(self._client.set_monitor_scale(output, scale)).result()

    def set_monitor_rotation(self, output: str, rotation: str) -> dict[str, Any]:
        return self._loop.submit(
            self._client.set_monitor_rotation(output, rotation)
        ).result()

    def enable_monitor(self, output: str) -> dict[str, Any]:
        return self._loop.submit(self._client.enable_monitor(output)).result()

    def disable_monitor(self, output: str) -> dict[str, Any]:
        return self._loop.submit(self._client.disable_monitor(output)).result()

    def wait_for(
        self,
        condition: str,
        params: dict[str, Any] | None = None,
        timeout_ms: int = 30_000,
        interval_ms: int | None = None,
    ) -> dict[str, Any]:
        return self._loop.submit(
            self._client.wait_for(
                condition,
                params=params,
                timeout_ms=timeout_ms,
                interval_ms=interval_ms,
            )
        ).result()

    def audit_log(
        self,
        limit: int | None = None,
        action_type: str | None = None,
        status: str | None = None,
    ) -> list[dict[str, Any]]:
        return self._loop.submit(
            self._client.audit_log(
                limit=limit,
                action_type=action_type,
                status=status,
            )
        ).result()

    def audit_clear(self) -> dict[str, Any]:
        return self._loop.submit(self._client.audit_clear()).result()

    def info(self) -> DaemonInfo:
        return self._loop.submit(self._client.info()).result()

    def inhibit_system(
        self,
        what: str,
        who: str = "deskbrid",
        why: str | None = None,
        mode: str | None = None,
    ) -> dict[str, Any]:
        return self._loop.submit(
            self._client.inhibit_system(what=what, who=who, why=why, mode=mode)
        ).result()

    def release_inhibit(self, inhibitor_id: int) -> dict[str, Any]:
        return self._loop.submit(self._client.release_inhibit(inhibitor_id)).result()

    def list_sessions(self) -> list[dict[str, Any]]:
        return self._loop.submit(self._client.list_sessions()).result()

    def lock_session(self, session_id: str | None = None) -> dict[str, Any]:
        return self._loop.submit(self._client.lock_session(session_id)).result()

    def switch_user(self, username: str) -> dict[str, Any]:
        return self._loop.submit(self._client.switch_user(username)).result()

    def check_auth(self, action_id: str) -> dict[str, Any]:
        return self._loop.submit(self._client.check_auth(action_id)).result()

    def elevate(self, action_id: str, reason: str | None = None) -> dict[str, Any]:
        return self._loop.submit(self._client.elevate(action_id, reason)).result()

    def confinement(self) -> dict[str, Any]:
        return self._loop.submit(self._client.confinement()).result()

    def service_status(self, name: str) -> dict[str, Any]:
        return self._loop.submit(self._client.service_status(name)).result()

    def service_start(self, name: str) -> dict[str, Any]:
        return self._loop.submit(self._client.service_start(name)).result()

    def service_stop(self, name: str) -> dict[str, Any]:
        return self._loop.submit(self._client.service_stop(name)).result()

    def service_restart(self, name: str) -> dict[str, Any]:
        return self._loop.submit(self._client.service_restart(name)).result()

    def service_enable(self, name: str, runtime: bool = False) -> dict[str, Any]:
        return self._loop.submit(self._client.service_enable(name, runtime)).result()

    def service_disable(self, name: str, runtime: bool = False) -> dict[str, Any]:
        return self._loop.submit(self._client.service_disable(name, runtime)).result()

    def service_list(self, unit_type: str | None = None) -> list[dict[str, Any]]:
        return self._loop.submit(self._client.service_list(unit_type)).result()

    def journal_query(
        self,
        since: int | None = None,
        until: int | None = None,
        unit: str | None = None,
        priority: int | None = None,
        tail: int | None = None,
    ) -> list[str]:
        return self._loop.submit(
            self._client.journal_query(
                since=since,
                until=until,
                unit=unit,
                priority=priority,
                tail=tail,
            )
        ).result()

    def timer_list(self) -> list[dict[str, Any]]:
        return self._loop.submit(self._client.timer_list()).result()

    def timer_start(self, name: str) -> dict[str, Any]:
        return self._loop.submit(self._client.timer_start(name)).result()

    def timer_stop(self, name: str) -> dict[str, Any]:
        return self._loop.submit(self._client.timer_stop(name)).result()

    def terminal_create(
        self,
        shell: str | None = None,
        cwd: str | None = None,
        env: dict[str, str] | None = None,
        rows: int | None = None,
        cols: int | None = None,
    ) -> dict[str, Any]:
        return self._loop.submit(
            self._client.terminal_create(
                shell=shell,
                cwd=cwd,
                env=env,
                rows=rows,
                cols=cols,
            )
        ).result()

    def terminal_write(self, terminal_id: str, input: str) -> dict[str, Any]:
        return self._loop.submit(self._client.terminal_write(terminal_id, input)).result()

    def terminal_read(
        self,
        terminal_id: str,
        max_bytes: int | None = None,
        flush: bool = True,
    ) -> dict[str, Any]:
        return self._loop.submit(
            self._client.terminal_read(
                terminal_id,
                max_bytes=max_bytes,
                flush=flush,
            )
        ).result()

    def terminal_resize(self, terminal_id: str, rows: int, cols: int) -> dict[str, Any]:
        return self._loop.submit(
            self._client.terminal_resize(terminal_id, rows, cols)
        ).result()

    def terminal_list(self) -> list[dict[str, Any]]:
        return self._loop.submit(self._client.terminal_list()).result()

    def terminal_kill(
        self,
        terminal_id: str,
        signal: str | None = None,
    ) -> dict[str, Any]:
        return self._loop.submit(self._client.terminal_kill(terminal_id, signal)).result()
