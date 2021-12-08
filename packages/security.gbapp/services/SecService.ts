const Fs = require('fs');
import urlJoin = require('url-join');

import { ConversationReference } from 'botbuilder';
import { GBLog, GBService, IGBInstance } from 'botlib';
import { CollectionUtil } from 'pragmatismo-io-framework';
import { GuaribasGroup, GuaribasUser, GuaribasUserGroup } from '../models';

/**
 * Security service layer.
 */
export class SecService extends GBService {
  public async importSecurityFile(localPath: string, instance: IGBInstance) {
    const security = JSON.parse(Fs.readFileSync(urlJoin(localPath, 'security.json'), 'utf8'));
    await CollectionUtil.asyncForEach(security.groups, async group => {
      const groupDb = GuaribasGroup.build({
        instanceId: instance.instanceId,
        displayName: group.displayName
      });
      const g1 = await groupDb.save();
      await CollectionUtil.asyncForEach(group.users, async user => {
        const userDb = GuaribasUser.build({
          instanceId: instance.instanceId,
          groupId: g1.groupId,
          userName: user.userName
        });
        const user2 = await userDb.save();
        const userGroup = GuaribasUserGroup.build();
        userGroup.groupId = g1.groupId;
        userGroup.userId = user2.userId;
        await userGroup.save();
      });
    });
  }

  public async ensureUser(
    instanceId: number,
    userSystemId: string,
    userName: string,
    address: string,
    channelName: string,
    displayName: string,
    email: string
  ): Promise<GuaribasUser> {
    let user = await GuaribasUser.findOne({
      where: {
        userSystemId: userSystemId
      }
    });

    if (!user) {
      user = GuaribasUser.build();
    }

    user.instanceId = instanceId;
    user.userSystemId = userSystemId;
    user.userName = userName;
    user.displayName = displayName;
    user.email = email;
    user.defaultChannel = channelName;

    return await user.save();
  }

  /**
   * Retrives a conversation reference from contact phone.
   */
  public async getConversationReference(phone: string): Promise<ConversationReference> {
    const options = { where: { phone: phone } };
    const user = await GuaribasUser.findOne(options);

    return JSON.parse(user.conversationReference);
  }

  /**
   * Updates a conversation reference from contact phone.
   */
  public async updateConversationReference(phone: string, conversationReference: string) {
    const options = { where: { phone: phone } };
    const user = await GuaribasUser.findOne(options);

    user.conversationReference = conversationReference;
    await user.save();
  }

  public async updateConversationReferenceById(userId: number, conversationReference: string) {
    const options = { where: { userId: userId } };
    const user = await GuaribasUser.findOne(options);

    user.conversationReference = conversationReference;
    await user.save();
  }

  public async updateUserLocale(userId: number, locale: any): Promise<GuaribasUser> {
    const user = await GuaribasUser.findOne({
      where: {
        userId: userId
      }
    });
    user.locale = locale;

    return await user.save();
  }

  public async updateUserHearOnDialog(userId: number, dialogName: string): Promise<GuaribasUser> {
    const user = await GuaribasUser.findOne({
      where: {
        userId: userId
      }
    });
    user.hearOnDialog = dialogName;

    return await user.save();
  }

  public async updateUserInstance(userSystemId: string, instanceId: number): Promise<GuaribasUser> {
    const user = await GuaribasUser.findOne({
      where: {
        userSystemId: userSystemId
      }
    });
    user.instanceId = instanceId;

    return await user.save();
  }

  /**
   * Finds and update user agent information to a next available person.
   */
  public async updateHumanAgent(
    userSystemId: string,
    instanceId: number,
    agentSystemId: string
  ): Promise<GuaribasUser> {
    const user = await GuaribasUser.findOne({
      where: {
        userSystemId: userSystemId,
        instanceId: instanceId
      }
    });

    if (agentSystemId === null  && user.agentSystemId !== undefined ) {
      const agent = await GuaribasUser.findOne({
        where: {
          userSystemId: user.agentSystemId
        }
      });

      if (agent !== null && agent !== undefined) {
        agent.agentMode = 'bot';
        agent.agentSystemId = null;
        await agent.save();
      }

      user.agentMode = 'bot';
      user.agentSystemId = null;
    } else {
      user.agentMode = 'human';
      user.agentSystemId = agentSystemId;
      const agent = await GuaribasUser.findOne({
        where: {
          userSystemId: agentSystemId
        }
      });

      agent.instanceId = user.instanceId;
      agent.agentMode = 'self';
      agent.agentSystemId = null;
      await agent.save();
    }

    await user.save();

    return user;
  }

  public async isAgentSystemId(systemId: string): Promise<Boolean> {
    const user = await GuaribasUser.findOne({
      where: {
        userSystemId: systemId
      }
    });

    if (user === null) {
      throw new Error(`TRANSFER_TO phones must talk first to the bot before becoming an agent.`);
    }

    return user.agentMode === 'self';
  }

  public async assignHumanAgent(userSystemId: string, instanceId: number): Promise<string> {
    let agentSystemId;
    const list = process.env.TRANSFER_TO.split(';');
    await CollectionUtil.asyncForEach(list, async item => {
      if (
        !(item !== undefined &&
          agentSystemId === undefined &&
          item !== userSystemId && await this.isAgentSystemId(item))
      ) {
        // TODO: Optimize loop.
        agentSystemId = item;
      }
    });
    GBLog.info(`Selected agentId: ${agentSystemId}`);
    await this.updateHumanAgent(userSystemId, instanceId, agentSystemId);
    GBLog.info(`Updated agentId to: ${agentSystemId}`);
    
    return agentSystemId;
  }

  public async getUserFromSystemId(systemId: string): Promise<GuaribasUser> {
    return await GuaribasUser.findOne({
      where: {
        userSystemId: systemId
      }
    });
  }

  public async getUserFromAgentSystemId(systemId: string): Promise<GuaribasUser> {
    return await GuaribasUser.findOne({
      where: {
        agentSystemId: systemId
      }
    });
  }

  public async getAllUsers(instanceId: number): Promise<GuaribasUser[]> {
    return await GuaribasUser.findAll({
      where: {
        instanceId: instanceId
      }
    });
  }
}
