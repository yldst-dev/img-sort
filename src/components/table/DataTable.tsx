import { ReactNode } from "react";

export interface Column<T> {
  key: string;
  header: string;
  render?: (row: T) => ReactNode;
}

interface DataTableProps<T> {
  columns: Column<T>[];
  data: T[];
  onRowClick?: (row: T) => void;
  emptyText?: string;
}

function DataTable<T>({ columns, data, onRowClick, emptyText = "No data" }: DataTableProps<T>) {
  return (
    <div className="table-container">
      <table className="data-table data-table-fixed">
        <thead>
          <tr>
            {columns.map((col) => (
              <th key={col.key}>
                <div className="cell">{col.header}</div>
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {data.length === 0 ? (
            <tr>
              <td colSpan={columns.length} className="empty-row">
                {emptyText}
              </td>
            </tr>
          ) : (
            data.map((row, idx) => (
              <tr key={idx} onClick={() => onRowClick?.(row)}>
                {columns.map((col) => (
                  <td key={col.key}>
                    <div className="cell">{col.render ? col.render(row) : (row as any)[col.key]}</div>
                  </td>
                ))}
              </tr>
            ))
          )}
        </tbody>
      </table>
    </div>
  );
}

export default DataTable;
