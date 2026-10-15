pub mod oauth;
pub mod graph;
pub mod sharepoint;
pub mod connector;
pub mod db;
pub mod storage;
pub mod handlers;
pub mod routes;
pub mod intent;

pub use intent::{M365Request, M365RequestKind};

pub use routes::configure;
pub use oauth::{M365OAuthFlow, M365Token, M365Scope, M365Error};
pub use graph::{GraphClient, GraphError, GraphSite, GraphList, GraphDriveItem};
pub use sharepoint::{SharePointSite, SharePointList, SharePointListItem, ListColumn};
pub use connector::{M365Source, M365SourceKind, M365SourceConfig, ConnectorError};
