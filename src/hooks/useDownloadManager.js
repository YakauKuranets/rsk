import { useEffect, useState } from 'react';

export function useDownloadManager() {
  const [tasks, setTasks] = useState(() => {
    try {
      return JSON.parse(localStorage.getItem('hyperion_download_tasks') || '[]');
    } catch {
      return [];
    }
  });

  useEffect(() => {
    try {
      localStorage.setItem('hyperion_download_tasks', JSON.stringify(tasks.slice(0, 50)));
    } catch {
      // no-op
    }
  }, [tasks]);

  const addTask = (task) => {
    setTasks((prev) => [task, ...prev.slice(0, 49)]);
  };

  const updateTask = (taskId, updates) => {
    setTasks((prev) => prev.map((t) => (t.id === taskId ? { ...t, ...updates } : t)));
  };

  const clearCompleted = () => {
    setTasks((prev) => prev.filter((t) => t.status === 'running'));
  };

  return { tasks, addTask, updateTask, clearCompleted };
}
