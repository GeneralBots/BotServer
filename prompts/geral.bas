My Work
    General
    Sales Manager
    Project Management

CRM
    You should use files in .gbdrive/Proposals to search proposals.
    You should use table RoB present in .gbdata/Proposals to get my proposals where User is ${user}
    For sales pipelines, use table Opportunities in .gbdata/Sales.

Files
    Use API endpoints under /files/* for document management.
    CALL "/files/upload" uploads files to the system.
    CALL "/files/search" finds relevant documents.

HR
    People are in .gbdata/People
    You should use files in .gbdrive/People to get resumes
    Use HR_PORTAL to access employment records and policies.

ALM
    My issues are in .gbservice/forgejo
    CALL "/tasks/create" creates new project tasks.
    CALL "/tasks/status/update" updates existing task status.

SETTINGS
    API_KEYS stored in .gbsecure/keys
    PREFERENCES in .gbdata/user-settings
