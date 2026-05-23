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
    fn file_list(&self, Parameters(FilePath { path }): Parameters<FilePath>) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "files.list",
            json!({"path": path}),
        )
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
    fn file_read(
        &self,
        Parameters(FileRead {
            path,
            offset,
            limit,
        }): Parameters<FileRead>,
    ) -> Json<Value> {
        let mut args = json!({"path": path});
        if let Some(o) = offset {
            args["offset"] = json!(o);
        }
        if let Some(l) = limit {
            args["limit"] = json!(l);
        }
        execute(self.state.clone(), &self.rt, "files.read", args)
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
    fn file_write(
        &self,
        Parameters(FileWrite {
            path,
            content,
            append,
        }): Parameters<FileWrite>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "files.write",
            json!({"path": path, "content": content, "append": append}),
        )
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
    fn file_search(
        &self,
        Parameters(FileSearch {
            pattern,
            root,
            max_results,
        }): Parameters<FileSearch>,
    ) -> Json<Value> {
        let mut args = json!({"pattern": pattern, "max_results": max_results});
        if let Some(r) = root {
            args["root"] = json!(r);
        }
        execute(self.state.clone(), &self.rt, "files.search", args)
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
    fn file_copy(
        &self,
        Parameters(FileCopy {
            source,
            destination,
        }): Parameters<FileCopy>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "files.copy",
            json!({"source": source, "destination": destination}),
        )
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
    fn file_watch(
        &self,
        Parameters(FileWatch {
            path,
            recursive,
            patterns,
        }): Parameters<FileWatch>,
    ) -> Json<Value> {
        let mut args = json!({"path": path, "recursive": recursive});
        if let Some(p) = patterns {
            args["patterns"] = json!(p);
        }
        execute(self.state.clone(), &self.rt, "files.watch", args)
    }
    };
}
