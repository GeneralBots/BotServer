

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


# 2.0.56

ALTER TABLE dbo.GuaribasUser ADD
	hearOnDialog nvarchar(64) NULL
GO


ALTER TABLE dbo.GuaribasConversation ADD
	instanceId int,
	feedback nvarchar(512) NULL
GO


ALTER TABLE [dbo].[GuaribasInstance] DROP COLUMN [translatorendpoint]
GO
ALTER TABLE dbo.GuaribasInstance ADD
	translatorEndpoint nvarchar(128) NULL
GO


# 2.0.108

ALTER TABLE [dbo].[GuaribasInstance] DROP COLUMN [agentSystemId]
GO

ALTER TABLE dbo.GuaribasUser ADD
	agentSystemId nvarchar(255) NULL,
GO

# 2.0.115

ALTER TABLE dbo.GuaribasQuestion ADD
	skipIndex bit NULL
GO

# 2.0.116 >


ALTER TABLE dbo.GuaribasInstance ADD
	googleBotKey nvarchar(255) NULL,
	googleChatApiKey nvarchar(255) NULL,
	googleChatSubscriptionName nvarchar(255) NULL,
	googleClientEmail  nvarchar(255) NULL,
	googlePrivateKey  nvarchar(4000) NULL,
	googleProjectId  nvarchar(255) NULL
GO

# 2.0.119

ALTER TABLE dbo.GuaribasInstance ADD
	facebookWorkplaceVerifyToken nvarchar(255) NULL,
	facebookWorkplaceAppSecret nvarchar(255) NULL,
	facebookWorkplaceAccessToken nvarchar(512) NULL
GO


# 2.0.140

/****** Object:  Table [dbo].[GuaribasSchedule]    Script Date: 25/08/2021 03:53:15  ******/
SET ANSI_NULLS ON
GO

SET QUOTED_IDENTIFIER ON
GO

CREATE TABLE [dbo].[GuaribasSchedule]
	[id] [int] IDENTITY(1,1) NOT NULL,
	[name] [nvarchar](255) NULL,
	[schedule] [nvarchar](255) NULL,
	[instanceId] [int] NULL,
	[createdAt] [datetimeoffset](7) NULL,
	[updatedAt] [datetimeoffset](7) NULL

GO


# 3.0.0

ALTER TABLE dbo.GuaribasInstance ADD botKey nvarchar(64) NULL;
