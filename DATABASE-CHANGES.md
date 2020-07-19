

# 2.0.0

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



ALTER TABLE [dbo].[GuaribasInstance] DROP COLUMN [authenticatorClientId]
GO

ALTER TABLE [dbo].[GuaribasInstance] DROP COLUMN [authenticatorClientSecret]
GO

ALTER TABLE dbo.GuaribasUser ADD
	locale nvarchar(5) NULL
GO


ALTER TABLE dbo.GuaribasInstance ADD
	translatorKey nvarchar(64) NULL
	translatorEndpoint nvarchar(64) NULL
GO


ALTER TABLE dbo.GuaribasInstance ADD
	activationCode nvarchar(16) NULL
GO

ALTER TABLE dbo.GuaribasInstance ADD
	params nvarchar(4000) NULL
GO

ALTER TABLE dbo.GuaribasInstance ADD
	state nvarchar(16) NULL
GO
UPDATE dbo.GuaribasInstance SET state= 'active' 

# 2.0.3

``` SQL

ALTER TABLE dbo.GuaribasPackage ADD
	params custom(512) NULL
GO

```