import axios from 'axios';

import { DataObject, OAuthResponse } from '../../lib/types';
import { convertToTimestamp } from '../../lib/helpers';

export const refresh = async ({ body }: DataObject): Promise<OAuthResponse> => {
    try {
        const {
            OAUTH_CLIENT_ID: client_id,
            OAUTH_CLIENT_SECRET: client_secret,
            OAUTH_REFRESH_TOKEN: refresh_token,
        } = body;

        const requestBody = {
            grant_type: 'refresh_token',
            client_id,
            client_secret,
            refresh_token,
        };

        const isSandbox = client_id.startsWith('sandbox-');
        const baseURL = isSandbox
            ? 'https://connect.squareupsandbox.com'
            : 'https://connect.squareup.com';

        const response = await axios.post(
            `${baseURL}/oauth2/token`,
            requestBody,
        );

        const {
            data: {
                access_token: accessToken,
                refresh_token: refreshToken,
                expires_at: expiresAt,
                token_type: tokenType,
                merchant_id: merchantId,
                short_lived: shortLived,
            },
        } = response;

        return {
            accessToken,
            refreshToken,
            expiresIn: Math.floor(
                ((await convertToTimestamp(expiresAt)) - new Date().getTime()) /
                    1000,
            ),
            tokenType: tokenType === 'bearer' ? 'Bearer' : tokenType,
            meta: {
                baseURL,
                merchantId,
                shortLived,
            },
        };
    } catch (error) {
        throw new Error(`Error fetching access token for Square: ${error}`);
    }
};
