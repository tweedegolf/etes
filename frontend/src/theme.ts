import { createTheme, MantineColorsTuple } from '@mantine/core';

const darkblue: MantineColorsTuple = [
  '#e6eaef',
  '#b3c1ce',
  '#8098ad',
  '#4d6e8c',
  '#1a456b',
  '#00305b',
  '#002649',
  '#001d37',
  '#001324',
  '#000a12'
];

const theme = createTheme({
  cursorType: 'pointer',
  primaryColor: 'darkblue',
  colors: {
    darkblue,
  }
});


export default theme;