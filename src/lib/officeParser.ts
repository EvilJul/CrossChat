import { invoke } from "@tauri-apps/api/core";
import * as XLSX from "xlsx";
import mammoth from "mammoth";

export interface ExcelSheetData {
  name: string;
  rows: string[][];
}

export interface ExcelPreviewResult {
  sheets: ExcelSheetData[];
}

export interface WordPreviewResult {
  html: string;
  text: string;
}

/**
 * Reads a local file as an ArrayBuffer using the custom Tauri command
 */
async function readFileAsArrayBuffer(filePath: string): Promise<ArrayBuffer> {
  const bytes = await invoke<number[]>("read_file_bytes", { path: filePath });
  const uint8Array = new Uint8Array(bytes);
  return uint8Array.buffer;
}

/**
 * Parses an Excel file and returns sheets with rows data
 */
export async function parseExcelFile(filePath: string): Promise<ExcelPreviewResult> {
  const arrayBuffer = await readFileAsArrayBuffer(filePath);
  const workbook = XLSX.read(arrayBuffer, { type: "array" });
  
  const sheets: ExcelSheetData[] = [];
  
  for (const sheetName of workbook.SheetNames) {
    const worksheet = workbook.Sheets[sheetName];
    // Convert to JSON array of arrays
    const json = XLSX.utils.sheet_to_json<any[]>(worksheet, { header: 1 });
    
    // Limit to max 100 rows to prevent frontend lag
    const limitedRows = json.slice(0, 100).map(row => 
      Array.isArray(row) 
        ? row.map(cell => cell !== undefined && cell !== null ? String(cell) : "") 
        : []
    );
    
    sheets.push({
      name: sheetName,
      rows: limitedRows
    });
  }
  
  return { sheets };
}

/**
 * Parses a Word document and returns HTML and plain text
 */
export async function parseWordFile(filePath: string): Promise<WordPreviewResult> {
  const arrayBuffer = await readFileAsArrayBuffer(filePath);
  const result = await mammoth.convertToHtml({ arrayBuffer });
  
  // Extract text by striping HTML
  const textResult = await mammoth.extractRawText({ arrayBuffer });
  
  return {
    html: result.value,
    text: textResult.value
  };
}
