#[macro_export]
macro_rules! tools_files {
    () => {

    #[tool(
        name = "file_list",
        description = "List files and directories at a path.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn file_list(&self, Parameters(FilePath { path }): Parameters<FilePath>) -> String {
        self.exec("files.list", json!({"path": path}),).await
    }

    #[tool(
        name = "file_read",
        description = "Read contents of a file.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn file_read(
        &self,
        Parameters(FileRead {
            path,
            offset,
            limit,
        }): Parameters<FileRead>,
    ) -> String {
        let mut args = json!({"path": path});
        if let Some(o) = offset {
            args["offset"] = json!(o);
        }
        if let Some(l) = limit {
            args["limit"] = json!(l);
        }
        self.exec("files.read", args).await
    }

    #[tool(
        name = "file_write",
        description = "Write content to a file (create or overwrite).",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn file_write(
        &self,
        Parameters(FileWrite {
            path,
            content,
            append,
        }): Parameters<FileWrite>,
    ) -> String {
        self.exec("files.write", json!({"path": path, "content": content, "append": append}),).await
    }

    #[tool(
        name = "file_search",
        description = "Search filesystem by glob or regex pattern.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn file_search(
        &self,
        Parameters(FileSearch {
            pattern,
            root,
            max_results,
        }): Parameters<FileSearch>,
    ) -> String {
        let mut args = json!({"pattern": pattern, "max_results": max_results});
        if let Some(r) = root {
            args["root"] = json!(r);
        }
        self.exec("files.search", args).await
    }

    #[tool(
        name = "file_copy",
        description = "Copy a file or directory.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn file_copy(
        &self,
        Parameters(FileCopy {
            source,
            destination,
        }): Parameters<FileCopy>,
    ) -> String {
        self.exec("files.copy", json!({"source": source, "destination": destination}),).await
    }

    #[tool(
        name = "file_watch",
        description = "Watch a path for file changes. Returns a watch ID.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn file_watch(
        &self,
        Parameters(FileWatch {
            path,
            recursive,
            patterns,
        }): Parameters<FileWatch>,
    ) -> String {
        let mut args = json!({"path": path, "recursive": recursive});
        if let Some(p) = patterns {
            args["patterns"] = json!(p);
        }
        self.exec("files.watch", args).await
    }
    };
}
