use super::*;

impl TrainingTracker {
    pub fn new() -> Self {
        let mut tracker = Self {
            courses: HashMap::new(),
            assignments: HashMap::new(),
            certificates: HashMap::new(),
            user_roles: HashMap::new(),
        };

        tracker.initialize_default_courses();
        tracker
    }

    pub(super) fn initialize_default_courses(&mut self) {
        let security_awareness = TrainingCourse {
            id: Uuid::new_v4(),
            title: "Security Awareness Fundamentals".to_string(),
            description: "Basic security awareness training for all employees".to_string(),
            training_type: TrainingType::SecurityAwareness,
            duration_hours: 2.0,
            validity_days: 365,
            priority: TrainingPriority::High,
            required_for_roles: vec!["all".to_string()],
            prerequisites: vec![],
            content_url: Some("https://training.example.com/security-awareness".to_string()),
            passing_score: 80,
            max_attempts: 3,
        };

        self.courses
            .insert(security_awareness.id, security_awareness);

        let data_protection = TrainingCourse {
            id: Uuid::new_v4(),
            title: "Data Protection and Privacy".to_string(),
            description: "Training on data protection regulations and best practices".to_string(),
            training_type: TrainingType::DataProtection,
            duration_hours: 3.0,
            validity_days: 365,
            priority: TrainingPriority::High,
            required_for_roles: vec!["admin".to_string(), "manager".to_string()],
            prerequisites: vec![],
            content_url: Some("https://training.example.com/data-protection".to_string()),
            passing_score: 85,
            max_attempts: 3,
        };

        self.courses.insert(data_protection.id, data_protection);
    }

    pub fn create_course(&mut self, course: TrainingCourse) -> Result<()> {
        if self.courses.contains_key(&course.id) {
            return Err(anyhow!("Course already exists"));
        }

        log::info!("Creating training course: {}", course.title);
        self.courses.insert(course.id, course);
        Ok(())
    }

    pub fn assign_training(
        &mut self,
        user_id: Uuid,
        course_id: Uuid,
        due_days: i64,
        assigned_by: String,
    ) -> Result<TrainingAssignment> {
        let course = self
            .courses
            .get(&course_id)
            .ok_or_else(|| anyhow!("Course not found"))?
            .clone();

        let assignment = TrainingAssignment {
            id: Uuid::new_v4(),
            user_id,
            course_id,
            assigned_date: Utc::now(),
            due_date: Utc::now() + Duration::days(due_days),
            status: TrainingStatus::NotStarted,
            attempts: vec![],
            completion_date: None,
            expiry_date: None,
            assigned_by,
            notes: None,
        };

        self.assignments.insert(assignment.id, assignment.clone());

        log::info!("Assigned training '{}' to user {}", course.title, user_id);

        Ok(assignment)
    }

    pub fn start_training(&mut self, assignment_id: Uuid) -> Result<TrainingAttempt> {
        let assignment = self
            .assignments
            .get_mut(&assignment_id)
            .ok_or_else(|| anyhow!("Assignment not found"))?;

        let course = self
            .courses
            .get(&assignment.course_id)
            .ok_or_else(|| anyhow!("Course not found"))?;

        if assignment.attempts.len() >= course.max_attempts as usize {
            return Err(anyhow!("Maximum attempts exceeded"));
        }

        let attempt = TrainingAttempt {
            id: Uuid::new_v4(),
            attempt_number: (assignment.attempts.len() + 1) as u32,
            start_time: Utc::now(),
            end_time: None,
            score: None,
            passed: false,
            time_spent_minutes: None,
        };

        assignment.status = TrainingStatus::InProgress;
        assignment.attempts.push(attempt.clone());

        Ok(attempt)
    }

    pub fn complete_training(
        &mut self,
        assignment_id: Uuid,
        attempt_id: Uuid,
        score: u32,
    ) -> Result<bool> {
        let (course_id, passing_score, validity_days, max_attempts, course_title) = {
            let assignment = self
                .assignments
                .get(&assignment_id)
                .ok_or_else(|| anyhow!("Assignment not found"))?;
            let course = self
                .courses
                .get(&assignment.course_id)
                .ok_or_else(|| anyhow!("Course not found"))?;
            (
                course.id,
                course.passing_score,
                course.validity_days,
                course.max_attempts,
                course.title.clone(),
            )
        };

        let assignment = self
            .assignments
            .get_mut(&assignment_id)
            .ok_or_else(|| anyhow!("Assignment not found"))?;

        let attempt_idx = assignment
            .attempts
            .iter()
            .position(|a| a.id == attempt_id)
            .ok_or_else(|| anyhow!("Attempt not found"))?;

        let end_time = Utc::now();
        let start_time = assignment.attempts[attempt_idx].start_time;
        let time_spent = (end_time - start_time).num_minutes() as u32;
        let passed = score >= passing_score;

        assignment.attempts[attempt_idx].end_time = Some(end_time);
        assignment.attempts[attempt_idx].score = Some(score);
        assignment.attempts[attempt_idx].time_spent_minutes = Some(time_spent);
        assignment.attempts[attempt_idx].passed = passed;

        let user_id = assignment.user_id;
        let attempts_count = assignment.attempts.len();

        if passed {
            assignment.status = TrainingStatus::Completed;
            assignment.completion_date = Some(end_time);
            assignment.expiry_date = Some(end_time + Duration::days(validity_days));

            let certificate = TrainingCertificate {
                id: Uuid::new_v4(),
                user_id,
                course_id,
                issued_date: end_time,
                expiry_date: end_time + Duration::days(validity_days),
                certificate_number: format!(
                    "CERT-{}",
                    Uuid::new_v4().to_string()[..8].to_uppercase()
                ),
                verification_code: Uuid::new_v4().to_string(),
            };

            self.certificates.insert(certificate.id, certificate);

            log::info!(
                "User {} completed training '{}' with score {}",
                user_id,
                course_title,
                score
            );
        } else if attempts_count >= max_attempts as usize {
            assignment.status = TrainingStatus::Failed;
        }

        Ok(passed)
    }

    pub fn get_compliance_status(&self, user_id: Uuid) -> ComplianceStatus {
        let user_roles = self
            .user_roles
            .get(&user_id)
            .cloned()
            .unwrap_or_else(|| vec!["all".to_string()]);

        let mut required_trainings = vec![];
        let mut completed_trainings = vec![];
        let mut overdue_trainings = vec![];
        let mut upcoming_trainings = vec![];

        for course in self.courses.values() {
            if course
                .required_for_roles
                .iter()
                .any(|r| user_roles.contains(r) || r == "all")
            {
                required_trainings.push(course.id);

                let assignment = self
                    .assignments
                    .values()
                    .find(|a| a.user_id == user_id && a.course_id == course.id);

                if let Some(assignment) = assignment {
                    match assignment.status {
                        TrainingStatus::Completed => {
                            if let Some(expiry) = assignment.expiry_date {
                                if expiry > Utc::now() {
                                    completed_trainings.push(course.id);
                                } else {
                                    overdue_trainings.push(course.id);
                                }
                            }
                        }
                        TrainingStatus::NotStarted | TrainingStatus::InProgress => {
                            if assignment.due_date < Utc::now() {
                                overdue_trainings.push(course.id);
                            } else {
                                upcoming_trainings.push(course.id);
                            }
                        }
                        _ => {}
                    }
                } else {
                    overdue_trainings.push(course.id);
                }
            }
        }

        let compliance_percentage = if required_trainings.is_empty() {
            100.0
        } else {
            (completed_trainings.len() as f64 / required_trainings.len() as f64) * 100.0
        };

        ComplianceStatus {
            user_id,
            compliant: overdue_trainings.is_empty(),
            required_trainings,
            completed_trainings,
            overdue_trainings,
            upcoming_trainings,
            compliance_percentage,
        }
    }

    pub fn get_training_report(&self) -> TrainingReport {
        let total_courses = self.courses.len();
        let total_assignments = self.assignments.len();
        let total_certificates = self.certificates.len();

        let mut assignments_by_status = HashMap::new();
        for assignment in self.assignments.values() {
            *assignments_by_status
                .entry(assignment.status.clone())
                .or_insert(0) += 1;
        }

        let overdue_count = self
            .assignments
            .values()
            .filter(|a| a.status != TrainingStatus::Completed && a.due_date < Utc::now())
            .count();

        let expiring_soon = self
            .certificates
            .values()
            .filter(|c| {
                c.expiry_date > Utc::now() && c.expiry_date < Utc::now() + Duration::days(30)
            })
            .count();

        let average_score = self.calculate_average_score();

        TrainingReport {
            generated_at: Utc::now(),
            total_courses,
            total_assignments,
            total_certificates,
            assignments_by_status,
            overdue_count,
            expiring_soon,
            average_score,
        }
    }

    pub(super) fn calculate_average_score(&self) -> f64 {
        let mut total_score = 0;
        let mut count = 0;

        for assignment in self.assignments.values() {
            for attempt in &assignment.attempts {
                if let Some(score) = attempt.score {
                    total_score += score;
                    count += 1;
                }
            }
        }

        if count == 0 {
            0.0
        } else {
            f64::from(total_score) / f64::from(count)
        }
    }

    pub fn set_user_roles(&mut self, user_id: Uuid, roles: Vec<String>) {
        self.user_roles.insert(user_id, roles);
    }

    pub fn get_overdue_trainings(&self) -> Vec<TrainingAssignment> {
        self.assignments
            .values()
            .filter(|a| a.status != TrainingStatus::Completed && a.due_date < Utc::now())
            .cloned()
            .collect()
    }

    pub fn get_expiring_certificates(&self, days_ahead: i64) -> Vec<TrainingCertificate> {
        let cutoff = Utc::now() + Duration::days(days_ahead);
        self.certificates
            .values()
            .filter(|c| c.expiry_date > Utc::now() && c.expiry_date <= cutoff)
            .cloned()
            .collect()
    }
}
