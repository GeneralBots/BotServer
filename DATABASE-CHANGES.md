

# 1.7.6
``` SQL
ALTER TABLE dbo.GuaribasUser ADD
	agentSystemId nvarchar(16) NULL,
	agentMode nvarchar(16) NULL,
	agentContacted datetime NULL
GO

ALTER TABLE [dbo].[GuaribasUser] DROP COLUMN [phone]
GO

ALTER TABLE [dbo].[GuaribasUser] DROP COLUMN [internalAddress]
GO

ALTER TABLE [dbo].[GuaribasUser] DROP COLUMN [currentBotId]
GO

```


# 1.7.7
``` SQL
ALTER TABLE [dbo].[GuaribasInstance] DROP COLUMN [authenticatorClientId]
GO

ALTER TABLE [dbo].[GuaribasInstance] DROP COLUMN [authenticatorClientSecret]
GO


```