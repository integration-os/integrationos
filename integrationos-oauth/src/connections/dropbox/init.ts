import axios from 'axios';
import { DataObject, OAuthResponse } from '../../lib/types';

export const init = async ({ body }: DataObject): Promise<OAuthResponse> => {
    try {
        const {
            clientId: client_id,
            clientSecret: client_secret,
            metadata: { code, redirectUri: redirect_uri },
        } = body;

        const response = await axios({
            url: 'https://api.dropboxapi.com/oauth2/token',
            method: 'POST',
            params: {
                grant_type: 'authorization_code',
                code,
                client_id,
                client_secret,
                redirect_uri,
            },
        });

        const {
            access_token: accessToken,
            refresh_token: refreshToken,
            token_type: tokenType,
            expires_in: expiresIn,
            scope,
            uid,
            account_id: accountId,
        } = response.data;

        return {
            accessToken,
            refreshToken,
            expiresIn,
            tokenType: tokenType === 'bearer' ? 'Bearer' : tokenType,
            meta: {
                scope,
                uid,
                accountId,
            },
        };
    } catch (error) {
        throw new Error(`Error fetching access token for Dropbox: ${error}`);
    }
};
