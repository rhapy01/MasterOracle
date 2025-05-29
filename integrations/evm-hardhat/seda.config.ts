export interface SedaConfig {
  coreAddress: string;
}

export const networkConfigs: { [network: string]: SedaConfig } = {
  // Proxy Core Addresses (SEDA testnet)
  baseSepolia: {
    coreAddress: '0xF631860f3Cb423aA14d06305083e4887e612A7f5',
  },
  superseedSepolia: {
    coreAddress: '0x752487C5daDD26D5E053EE50b62fFd8A06eCAE47',
  },
};
