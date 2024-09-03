import axios from 'axios';
import { DataObject, OAuthResponse } from '../../lib/types';
import { generateBasicHeaders } from '../../lib/helpers';

export const refresh = async ({ body }: DataObject): Promise<OAuthResponse> => {
    try {
        const {
            OAUTH_CLIENT_ID: client_id,
            OAUTH_CLIENT_SECRET: client_secret,
            OAUTH_REFRESH_TOKEN: refresh_token,
        } = body;

        const requestBody = {
            grant_type: 'refresh_token',
            refresh_token,
        };

        const response = await axios.post(
            'https://oauth.platform.intuit.com/oauth2/v1/tokens/bearer',
            requestBody,
            {
                headers: {
                    ...generateBasicHeaders(client_id, client_secret),
                    Accept: 'application/json',
                },
            },
        );

        const {
            data: {
                access_token: accessToken,
                refresh_token: refreshToken,
                expires_in: expiresIn,
                token_type: tokenType,
            },
        } = response;

        return {
            accessToken,
            refreshToken,
            expiresIn,
            tokenType,
            meta: {
                ...body?.OAUTH_METADATA?.meta,
            },
        };
    } catch (error) {
        throw new Error(`Error fetching access token for Quickbooks: ${error}`);
    }
};
