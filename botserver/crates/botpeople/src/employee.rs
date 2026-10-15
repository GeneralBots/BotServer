use chrono::{NaiveDate, Utc};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use crate::models::{Employee, EmploymentStatus};

type Storage = Arc<Mutex<HashMap<Uuid, Employee>>>;

/// A new employee record.
pub struct NewEmployee {
    pub org_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub department: String,
    pub position: String,
    pub hire_date: NaiveDate,
    pub salary: f64,
    pub manager_id: Option<Uuid>,
}

/// The fields one update may change; a `None` field keeps its current value.
#[derive(Default)]
pub struct EmployeeUpdate {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub department: Option<String>,
    pub position: Option<String>,
    pub salary: Option<f64>,
    pub status: Option<String>,
    pub manager_id: Option<Uuid>,
}

#[derive(Clone)]
pub struct EmployeeService {
    storage: Storage,
}

impl EmployeeService {
    pub fn new() -> Self {
        EmployeeService {
            storage: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn create(&self, new_employee: NewEmployee) -> Result<Employee, String> {
        let NewEmployee {
            org_id,
            first_name,
            last_name,
            email,
            phone,
            department,
            position,
            hire_date,
            salary,
            manager_id,
        } = new_employee;
        let id = Uuid::new_v4();
        let now = Utc::now();
        let employee = Employee {
            id,
            org_id,
            first_name,
            last_name,
            email,
            phone,
            department,
            position,
            hire_date,
            status: EmploymentStatus::Active,
            salary,
            manager_id,
            created_at: now,
            updated_at: now,
        };
        let mut store = self.storage.lock().map_err(|e| format!("Lock error: {e}"))?;
        store.insert(id, employee.clone());
        Ok(employee)
    }

    pub fn get(&self, id: Uuid) -> Result<Employee, String> {
        let store = self.storage.lock().map_err(|e| format!("Lock error: {e}"))?;
        store.get(&id).cloned().ok_or_else(|| format!("Employee not found: {id}"))
    }

    pub fn list(&self) -> Result<Vec<Employee>, String> {
        let store = self.storage.lock().map_err(|e| format!("Lock error: {e}"))?;
        let mut employees: Vec<Employee> = store.values().cloned().collect();
        employees.sort_by(|a, b| a.last_name.cmp(&b.last_name));
        Ok(employees)
    }

    pub fn search_by_department(&self, department: &str) -> Result<Vec<Employee>, String> {
        let store = self.storage.lock().map_err(|e| format!("Lock error: {e}"))?;
        let employees: Vec<Employee> = store
            .values()
            .filter(|e| e.department == department)
            .cloned()
            .collect();
        Ok(employees)
    }

    pub fn search_by_status(&self, status: EmploymentStatus) -> Result<Vec<Employee>, String> {
        let store = self.storage.lock().map_err(|e| format!("Lock error: {e}"))?;
        let employees: Vec<Employee> = store
            .values()
            .filter(|e| e.status == status)
            .cloned()
            .collect();
        Ok(employees)
    }

    pub fn update(&self, id: Uuid, changes: EmployeeUpdate) -> Result<Employee, String> {
        let EmployeeUpdate {
            first_name,
            last_name,
            email,
            phone,
            department,
            position,
            salary,
            status,
            manager_id,
        } = changes;
        let mut store = self.storage.lock().map_err(|e| format!("Lock error: {e}"))?;
        let emp = store.get_mut(&id).ok_or_else(|| format!("Employee not found: {id}"))?;
        if let Some(fn_val) = first_name {
            emp.first_name = fn_val;
        }
        if let Some(ln_val) = last_name {
            emp.last_name = ln_val;
        }
        if let Some(e_val) = email {
            emp.email = e_val;
        }
        if let Some(p_val) = phone {
            emp.phone = Some(p_val);
        }
        if let Some(d_val) = department {
            emp.department = d_val;
        }
        if let Some(pos_val) = position {
            emp.position = pos_val;
        }
        if let Some(s_val) = salary {
            emp.salary = s_val;
        }
        if let Some(ref s) = status {
            if let Some(st) = EmploymentStatus::from_label(s) {
                emp.status = st;
            }
        }
        if let Some(m_val) = manager_id {
            emp.manager_id = Some(m_val);
        }
        emp.updated_at = Utc::now();
        Ok(emp.clone())
    }

    pub fn delete(&self, id: Uuid) -> Result<(), String> {
        let mut store = self.storage.lock().map_err(|e| format!("Lock error: {e}"))?;
        store.remove(&id).ok_or_else(|| format!("Employee not found: {id}"))?;
        Ok(())
    }

    pub fn hierarchy_tree(&self, manager_id: Uuid) -> Result<Vec<EmployeeHierarchyNode>, String> {
        let store = self.storage.lock().map_err(|e| format!("Lock error: {e}"))?;
        let root = store.get(&manager_id).cloned()
            .ok_or_else(|| format!("Manager not found: {manager_id}"))?;
        drop(store);
        let children = self.get_direct_reports(manager_id)?;
        let mut child_nodes: Vec<EmployeeHierarchyNode> = Vec::new();
        for child in &children {
            let grandchildren = self.get_direct_reports(child.id)?;
            let grandchild_nodes: Vec<EmployeeHierarchyNode> = grandchildren
                .into_iter()
                .map(|gc| EmployeeHierarchyNode {
                    employee: gc,
                    children: Vec::new(),
                })
                .collect();
            child_nodes.push(EmployeeHierarchyNode {
                employee: child.clone(),
                children: grandchild_nodes,
            });
        }
        Ok(vec![EmployeeHierarchyNode {
            employee: root,
            children: child_nodes,
        }])
    }

    fn get_direct_reports(&self, manager_id: Uuid) -> Result<Vec<Employee>, String> {
        let store = self.storage.lock().map_err(|e| format!("Lock error: {e}"))?;
        let reports: Vec<Employee> = store
            .values()
            .filter(|e| e.manager_id == Some(manager_id) && e.status == EmploymentStatus::Active)
            .cloned()
            .collect();
        Ok(reports)
    }
}

impl Default for EmployeeService {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct EmployeeHierarchyNode {
    pub employee: Employee,
    pub children: Vec<EmployeeHierarchyNode>,
}
