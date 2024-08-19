export interface IUser {
    id: string,
    name: string
}

export interface IChannelAccount {
    id?: string,
    name?: string,
}

export interface IConversationAccount extends IChannelAccount {
    isGroup?: boolean,
}

export interface IAttachment {
    contentType?: string,
    contentUrl?: string,
    content?: any,
    name?: string,
    thumbnailUrl?: string,
}

export interface IEntity {
    type?: string,
}

export interface IActivity {
    type?: string,
    id?: string,
    serviceUrl?: string,
    timestamp?: string,
    localTimestamp?: string,
    channelId?: string,
    from?: IChannelAccount,
    conversation?: IConversationAccount,
    recipient?: IChannelAccount,
    replyToId?: string,
    channelData?: any,
}

export interface IMessageActivity extends IActivity {
    locale?: string,
    text?: string,
    summary?: string,
    textFormat?: string,
    attachmentLayout?: string,
    attachments?: IAttachment[],
    entities?: IEntity[],
}

export interface IBotData {
    eTag: string;
    data: any;
}

export interface IConversation {
    conversationId: string,
    history?: IActivity[]
}

export interface IConversationUpdateActivity extends IActivity {
    membersAdded?: IChannelAccount[],
    membersRemoved?: IChannelAccount[],
    topicName?: string,
    historyDisclosed?: boolean,
}