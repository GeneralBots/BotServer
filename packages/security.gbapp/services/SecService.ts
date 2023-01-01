import Fs from 'fs';
import urlJoin from 'url-join';

import { ConversationReference } from 'botbuilder';
import { GBLog, GBMinInstance, GBService, IGBInstance } from 'botlib';
import { CollectionUtil } from 'pragmatismo-io-framework';
import { GuaribasGroup, GuaribasUser, GuaribasUserGroup } from '../models/index.js';
import { FindOptions } from 'sequelize';

/**
 * Security service layer.
 */
export class SecService extends GBService {
  public async ensureUser (
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
  public async getConversationReference (phone: string): Promise<ConversationReference> {
    const options = <FindOptions>{ rejectOnEmpty: true, where: { phone: phone } };
    const user = await GuaribasUser.findOne(options);

    return JSON.parse(user.conversationReference);
  }

  /**
   * Updates a conversation reference from contact phone.
   */
  public async updateConversationReference (phone: string, conversationReference: string) {
    const options = <FindOptions>{ where: { phone: phone } };
    const user = await GuaribasUser.findOne(options);

    user.conversationReference = conversationReference;
    await user.save();
  }

  public async updateConversationReferenceById (userId: number, conversationReference: string) {
    const options = <FindOptions>{ where: { userId: userId } };
    const user = await GuaribasUser.findOne(options);

    user.conversationReference = conversationReference;
    await user.save();
  }

  public async updateUserLocale (userId: number, locale: any): Promise<GuaribasUser> {
    const user = await GuaribasUser.findOne({
      where: {
        userId: userId
      }
    });
    user.locale = locale;

    return await user.save();
  }

  public async updateUserHearOnDialog (userId: number, dialogName: string): Promise<GuaribasUser> {
    const user = await GuaribasUser.findOne({
      where: {
        userId: userId
      }
    });
    user.hearOnDialog = dialogName;

    return await user.save();
  }

  public async updateUserInstance (userSystemId: string, instanceId: number): Promise<GuaribasUser> {
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
  public async updateHumanAgent (
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

    if (agentSystemId === null && user.agentSystemId !== undefined) {
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

  public async isAgentSystemId (systemId: string): Promise<Boolean> {
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

  public async assignHumanAgent (
    min: GBMinInstance,
    userSystemId: string,
    agentSystemId: string = null
  ): Promise<string> {
    if (!agentSystemId) {
      let list = min.core.getParam<string>(min.instance, 'Transfer To', process.env.TRANSFER_TO);

      if (list) {
        list = list.split(';');
      }

      await CollectionUtil.asyncForEach(list, async item => {
        if (
          item !== undefined &&
          agentSystemId === undefined &&
          item !== userSystemId &&
          !(await this.isAgentSystemId(item))
        ) {
          agentSystemId = item;
        }
      });
    }
    GBLog.info(`Selected agentId: ${agentSystemId}`);
    await this.updateHumanAgent(userSystemId, min.instance.instanceId, agentSystemId);
    GBLog.info(`Updated agentId to: ${agentSystemId}`);

    return agentSystemId;
  }

  public async getUserFromSystemId (systemId: string): Promise<GuaribasUser> {
    return await GuaribasUser.findOne({
      where: {
        userSystemId: systemId
      }
    });
  }

  public async getUserFromAgentSystemId (systemId: string): Promise<GuaribasUser> {
    return await GuaribasUser.findOne({
      where: {
        agentSystemId: systemId
      }
    });
  }

  public async getAllUsers (instanceId: number): Promise<GuaribasUser[]> {
    return await GuaribasUser.findAll({
      where: {
        instanceId: instanceId
      }
    });
  }
}
