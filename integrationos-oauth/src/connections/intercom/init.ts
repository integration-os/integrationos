import axios from 'axios';
import { DataObject, OAuthResponse } from '../../lib/types';

export const init = async ({ body }: DataObject): Promise<OAuthResponse> => {
    try {
        const {
            clientId: client_id,
            clientSecret: client_secret,
            metadata: { code },
        } = body;

        const response = await axios({
            url: 'https://api.intercom.io/auth/eagle/token',
            method: 'POST',
            params: {
                code,
                client_id,
                client_secret,
            },
        });

        const { access_token: accessToken, token_type: tokenType } =
            response.data;

        return {
            accessToken,
            refreshToken: '',
            expiresIn: 2147483647,
            tokenType: tokenType === 'bearer' ? 'Bearer' : tokenType,
            meta: {},
        };
    } catch (error) {
        throw new Error(`Error fetching access token for Intercom: ${error}`);
    }
};
