//! DAG (Directed Acyclic Graph) engine for task dependency resolution

use crate::error::{PicoFlowError, Result};
use crate::models::TaskConfig;
use petgraph::algo::{is_cyclic_directed, toposort};
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;

/// DAG (Directed Acyclic Graph) engine for workflow task management.
///
/// The DAG engine builds and validates task dependency graphs using petgraph.
/// It provides topological sorting for sequential execution and parallel level
/// computation for concurrent execution in Phase 3.
///
/// # Performance
///
/// Target: <50ms for 100 tasks (PRD PERF-005)
#[derive(Debug)]
pub struct DagEngine {
    graph: DiGraph<String, ()>,
    task_indices: HashMap<String, NodeIndex>,
}

impl DagEngine {
    /// Build and validate a DAG from task configurations.
    ///
    /// This method constructs a directed graph from task dependencies and validates
    /// that the graph is acyclic (no circular dependencies). If a cycle is detected,
    /// the error message includes the cycle path for debugging.
    ///
    /// # Arguments
    ///
    /// * `tasks` - Array of task configurations with dependencies
    ///
    /// # Returns
    ///
    /// * `Ok(DagEngine)` - Validated DAG engine ready for execution planning
    ///
    /// # Errors
    ///
    /// * `PicoFlowError::CycleDetected` - If circular dependencies are found
    ///
    /// # Example
    ///
    /// ```
    /// use picoflow::dag::DagEngine;
    /// use picoflow::models::{TaskConfig, TaskType, TaskExecutorConfig, ShellConfig};
    ///
    /// let tasks = vec![
    ///     TaskConfig {
    ///         name: "task_a".to_string(),
    ///         task_type: TaskType::Shell,
    ///         depends_on: vec![],
    ///         config: TaskExecutorConfig::Shell(ShellConfig {
    ///             command: "/bin/echo".to_string(),
    ///             args: vec!["hello".to_string()],
    ///             workdir: None,
    ///             env: None,
    ///         }),
    ///         retry: Some(3),
    ///         timeout: Some(300),
    ///         continue_on_failure: false,
    ///     },
    /// ];
    ///
    /// let dag = DagEngine::build(&tasks)?;
    /// # Ok::<(), picoflow::error::PicoFlowError>(())
    /// ```
    pub fn build(tasks: &[TaskConfig]) -> Result<Self> {
        let mut graph = DiGraph::new();
        let mut task_indices = HashMap::new();

        // Create nodes for all tasks
        for task in tasks {
            let index = graph.add_node(task.name.clone());
            task_indices.insert(task.name.clone(), index);
        }

        // Create edges from dependencies
        for task in tasks {
            let task_index = task_indices[&task.name];
            for dep_name in &task.depends_on {
                // Validate that the dependency task exists
                let dep_index = match task_indices.get(dep_name) {
                    Some(&index) => index,
                    None => {
                        return Err(PicoFlowError::MissingDependency {
                            task: task.name.clone(),
                            dependency: dep_name.clone(),
                        });
                    }
                };
                // Edge from dependency to task (dep must complete before task)
                graph.add_edge(dep_index, task_index, ());
            }
        }

        let engine = Self {
            graph,
            task_indices,
        };

        // Validate DAG is acyclic
        engine.validate_acyclic()?;

        Ok(engine)
    }

    /// Validate that the graph contains no cycles.
    ///
    /// This method checks for circular dependencies in the task graph. If a cycle
    /// is found, it uses DFS to locate and report the cycle path in the error message.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If graph is acyclic (valid DAG)
    ///
    /// # Errors
    ///
    /// * `PicoFlowError::CycleDetected` - If circular dependencies exist, with cycle path
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use picoflow::dag::DagEngine;
    /// # use picoflow::models::TaskConfig;
    /// # let tasks: Vec<TaskConfig> = vec![];
    /// let dag = DagEngine::build(&tasks)?;
    /// dag.validate_acyclic()?; // Called automatically by build()
    /// # Ok::<(), picoflow::error::PicoFlowError>(())
    /// ```
    pub fn validate_acyclic(&self) -> Result<()> {
        if is_cyclic_directed(&self.graph) {
            // Find a cycle for better error message
            let cycle_info = self.find_cycle();
            return Err(PicoFlowError::CycleDetected(cycle_info));
        }
        Ok(())
    }

    /// Find a cycle in the graph for error reporting
    fn find_cycle(&self) -> String {
        // Simple DFS to find cycle
        let mut visited = HashMap::new();
        let mut path = Vec::new();

        for node in self.graph.node_indices() {
            if !visited.contains_key(&node) {
                if let Some(cycle) = self.dfs_find_cycle(node, &mut visited, &mut path) {
                    return cycle;
                }
            }
        }

        "Unknown cycle".to_string()
    }

    fn dfs_find_cycle(
        &self,
        node: NodeIndex,
        visited: &mut HashMap<NodeIndex, bool>,
        path: &mut Vec<String>,
    ) -> Option<String> {
        if let Some(&in_path) = visited.get(&node) {
            if in_path {
                // Found cycle
                let task_name = &self.graph[node];
                path.push(task_name.clone());
                return Some(path.join(" -> "));
            }
            return None;
        }

        visited.insert(node, true);
        path.push(self.graph[node].clone());

        for neighbor in self.graph.neighbors(node) {
            if let Some(cycle) = self.dfs_find_cycle(neighbor, visited, path) {
                return Some(cycle);
            }
        }

        path.pop();
        visited.insert(node, false);
        None
    }

    /// Get task names in topologically sorted order for sequential execution.
    ///
    /// Returns tasks ordered such that all dependencies of a task appear before
    /// the task itself. This ordering is used by the scheduler for sequential execution.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<String>)` - Task names in dependency-safe execution order
    ///
    /// # Errors
    ///
    /// * `PicoFlowError::CycleDetected` - If graph contains cycles (should not occur after build)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use picoflow::dag::DagEngine;
    /// # use picoflow::models::TaskConfig;
    /// # let tasks: Vec<TaskConfig> = vec![];
    /// let dag = DagEngine::build(&tasks)?;
    /// let execution_order = dag.topological_sort()?;
    /// for task_name in execution_order {
    ///     println!("Execute: {}", task_name);
    /// }
    /// # Ok::<(), picoflow::error::PicoFlowError>(())
    /// ```
    pub fn topological_sort(&self) -> Result<Vec<String>> {
        let sorted_indices = toposort(&self.graph, None).map_err(|_| {
            PicoFlowError::CycleDetected("Cycle detected during topological sort".to_string())
        })?;

        Ok(sorted_indices
            .iter()
            .map(|&idx| self.graph[idx].clone())
            .collect())
    }

    /// Get tasks grouped by execution level for parallel execution (Phase 3).
    ///
    /// Returns tasks organized into execution levels where all tasks in a level
    /// can be executed in parallel:
    /// - Level 0: Tasks with no dependencies
    /// - Level 1: Tasks depending only on level 0 tasks
    /// - Level N: Tasks whose deepest dependency is at level N-1
    ///
    /// # Returns
    ///
    /// * `Vec<Vec<String>>` - Array of execution levels, each containing task names
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use picoflow::dag::DagEngine;
    /// # use picoflow::models::TaskConfig;
    /// # let tasks: Vec<TaskConfig> = vec![];
    /// let dag = DagEngine::build(&tasks)?;
    /// let levels = dag.parallel_levels();
    /// for (level_num, level_tasks) in levels.iter().enumerate() {
    ///     println!("Level {}: can run {} tasks in parallel", level_num, level_tasks.len());
    /// }
    /// # Ok::<(), picoflow::error::PicoFlowError>(())
    /// ```
    pub fn parallel_levels(&self) -> Vec<Vec<String>> {
        let mut levels: Vec<Vec<String>> = Vec::new();
        let node_count = self.graph.node_count();
        let mut node_levels: HashMap<NodeIndex, usize> = HashMap::with_capacity(node_count);

        // Calculate level for each node
        for node in self.graph.node_indices() {
            let level = self.calculate_node_level(node, &mut node_levels);
            node_levels.insert(node, level);
        }

        // Group nodes by level
        for (node, level) in node_levels {
            while levels.len() <= level {
                levels.push(Vec::new());
            }
            levels[level].push(self.graph[node].clone());
        }

        levels
    }

    fn calculate_node_level(
        &self,
        node: NodeIndex,
        cache: &mut HashMap<NodeIndex, usize>,
    ) -> usize {
        if let Some(&level) = cache.get(&node) {
            return level;
        }

        let mut max_dep_level = 0;
        for parent in self
            .graph
            .neighbors_directed(node, petgraph::Direction::Incoming)
        {
            let parent_level = self.calculate_node_level(parent, cache);
            max_dep_level = max_dep_level.max(parent_level + 1);
        }

        cache.insert(node, max_dep_level);
        max_dep_level
    }

    /// Get all tasks that directly depend on the given task.
    ///
    /// Returns the immediate children of a task in the dependency graph
    /// (tasks that will be unblocked when this task completes).
    ///
    /// # Arguments
    ///
    /// * `task_name` - Name of the task to query
    ///
    /// # Returns
    ///
    /// * `Vec<String>` - Names of tasks that depend on this task (empty if none or task not found)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use picoflow::dag::DagEngine;
    /// # use picoflow::models::TaskConfig;
    /// # let tasks: Vec<TaskConfig> = vec![];
    /// let dag = DagEngine::build(&tasks)?;
    /// let dependents = dag.get_dependents("task_a");
    /// println!("Tasks waiting for task_a: {:?}", dependents);
    /// # Ok::<(), picoflow::error::PicoFlowError>(())
    /// ```
    pub fn get_dependents(&self, task_name: &str) -> Vec<String> {
        if let Some(&index) = self.task_indices.get(task_name) {
            self.graph
                .neighbors(index)
                .map(|idx| self.graph[idx].clone())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get all tasks that the given task directly depends on.
    ///
    /// Returns the immediate parents of a task in the dependency graph
    /// (tasks that must complete before this task can start).
    ///
    /// # Arguments
    ///
    /// * `task_name` - Name of the task to query
    ///
    /// # Returns
    ///
    /// * `Vec<String>` - Names of tasks this task depends on (empty if none or task not found)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use picoflow::dag::DagEngine;
    /// # use picoflow::models::TaskConfig;
    /// # let tasks: Vec<TaskConfig> = vec![];
    /// let dag = DagEngine::build(&tasks)?;
    /// let dependencies = dag.get_dependencies("task_c");
    /// println!("task_c requires: {:?}", dependencies);
    /// # Ok::<(), picoflow::error::PicoFlowError>(())
    /// ```
    pub fn get_dependencies(&self, task_name: &str) -> Vec<String> {
        if let Some(&index) = self.task_indices.get(task_name) {
            self.graph
                .neighbors_directed(index, petgraph::Direction::Incoming)
                .map(|idx| self.graph[idx].clone())
                .collect()
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ShellConfig, TaskExecutorConfig, TaskType};

    fn create_test_task(name: &str, depends_on: Vec<String>) -> TaskConfig {
        TaskConfig {
            name: name.to_string(),
            task_type: TaskType::Shell,
            depends_on,
            config: TaskExecutorConfig::Shell(ShellConfig {
                command: "/bin/true".to_string(),
                args: vec![],
                workdir: None,
                env: None,
            }),
            retry: Some(3),
            timeout: Some(300),
            continue_on_failure: false,
        }
    }

    #[test]
    fn test_simple_dag() {
        let tasks = vec![
            create_test_task("a", vec![]),
            create_test_task("b", vec!["a".to_string()]),
            create_test_task("c", vec!["b".to_string()]),
        ];

        let dag = DagEngine::build(&tasks).unwrap();
        let sorted = dag.topological_sort().unwrap();

        assert_eq!(sorted.len(), 3);
        assert_eq!(sorted[0], "a");
        assert_eq!(sorted[1], "b");
        assert_eq!(sorted[2], "c");
    }

    #[test]
    fn test_parallel_dag() {
        let tasks = vec![
            create_test_task("a", vec![]),
            create_test_task("b", vec!["a".to_string()]),
            create_test_task("c", vec!["a".to_string()]),
            create_test_task("d", vec!["b".to_string(), "c".to_string()]),
        ];

        let dag = DagEngine::build(&tasks).unwrap();
        let levels = dag.parallel_levels();

        assert_eq!(levels.len(), 3);
        assert_eq!(levels[0], vec!["a"]);
        assert!(levels[1].contains(&"b".to_string()));
        assert!(levels[1].contains(&"c".to_string()));
        assert_eq!(levels[2], vec!["d"]);
    }

    #[test]
    fn test_cyclic_dag() {
        let tasks = vec![
            create_test_task("a", vec!["c".to_string()]),
            create_test_task("b", vec!["a".to_string()]),
            create_test_task("c", vec!["b".to_string()]),
        ];

        let result = DagEngine::build(&tasks);
        assert!(matches!(result, Err(PicoFlowError::CycleDetected(_))));
    }

    #[test]
    fn test_self_cycle() {
        let tasks = vec![create_test_task("a", vec!["a".to_string()])];

        let result = DagEngine::build(&tasks);
        assert!(matches!(result, Err(PicoFlowError::CycleDetected(_))));
    }

    #[test]
    fn test_missing_dependency() {
        let tasks = vec![
            create_test_task("a", vec![]),
            create_test_task("b", vec!["nonexistent".to_string()]),
        ];

        let result = DagEngine::build(&tasks);
        assert!(matches!(
            result,
            Err(PicoFlowError::MissingDependency { task, dependency })
            if task == "b" && dependency == "nonexistent"
        ));
    }

    #[test]
    fn test_disconnected_graph() {
        let tasks = vec![
            create_test_task("a", vec![]),
            create_test_task("b", vec![]),
            create_test_task("c", vec![]),
        ];

        let dag = DagEngine::build(&tasks).unwrap();
        let sorted = dag.topological_sort().unwrap();

        assert_eq!(sorted.len(), 3);
    }

    #[test]
    fn test_get_dependencies() {
        let tasks = vec![
            create_test_task("a", vec![]),
            create_test_task("b", vec!["a".to_string()]),
            create_test_task("c", vec!["a".to_string(), "b".to_string()]),
        ];

        let dag = DagEngine::build(&tasks).unwrap();

        let deps_a = dag.get_dependencies("a");
        assert_eq!(deps_a.len(), 0);

        let deps_b = dag.get_dependencies("b");
        assert_eq!(deps_b.len(), 1);
        assert!(deps_b.contains(&"a".to_string()));

        let deps_c = dag.get_dependencies("c");
        assert_eq!(deps_c.len(), 2);
        assert!(deps_c.contains(&"a".to_string()));
        assert!(deps_c.contains(&"b".to_string()));
    }

    #[test]
    fn test_get_dependents() {
        let tasks = vec![
            create_test_task("a", vec![]),
            create_test_task("b", vec!["a".to_string()]),
            create_test_task("c", vec!["a".to_string()]),
        ];

        let dag = DagEngine::build(&tasks).unwrap();

        let dependents_a = dag.get_dependents("a");
        assert_eq!(dependents_a.len(), 2);
        assert!(dependents_a.contains(&"b".to_string()));
        assert!(dependents_a.contains(&"c".to_string()));

        let dependents_b = dag.get_dependents("b");
        assert_eq!(dependents_b.len(), 0);
    }

    #[test]
    fn test_complex_dag() {
        // Diamond shape: a -> b,c -> d
        let tasks = vec![
            create_test_task("a", vec![]),
            create_test_task("b", vec!["a".to_string()]),
            create_test_task("c", vec!["a".to_string()]),
            create_test_task("d", vec!["b".to_string(), "c".to_string()]),
            create_test_task("e", vec!["d".to_string()]),
        ];

        let dag = DagEngine::build(&tasks).unwrap();
        let levels = dag.parallel_levels();

        assert_eq!(levels.len(), 4);
        assert_eq!(levels[0], vec!["a"]);
        assert_eq!(levels[1].len(), 2); // b and c in parallel
        assert_eq!(levels[2], vec!["d"]);
        assert_eq!(levels[3], vec!["e"]);
    }
}
